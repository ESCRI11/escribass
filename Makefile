# Shortcuts, not a build system.
#
# Every recipe here shells out to `cargo`, `npm` or a script in `app/package.json`, which stay
# the source of truth for how anything is built. Nothing is reimplemented, and **CI does not
# call any of these targets** — it runs the same commands directly, on purpose: two
# descriptions of one build drift the day either changes, and a Makefile that CI depends on is
# a second build system whether or not it was meant as one.
#
# `make` on its own lists what there is.

# rustup's shim first, because `rust-toolchain.toml` only binds through it. Ubuntu ships a
# `cargo` at /usr/bin, it is 1.75.0, and `Cargo.lock` is version 4 — so a plain login shell
# runs the distro cargo and every target that touches Rust dies with "lock file version 4
# requires -Znext-lockfile-bump". `$(HOME)/.cargo/bin/cargo` is rustup's shim: it reads
# `rust-toolchain.toml` and hands over to the pinned 1.98.0, which is the entire reason that
# file exists. Do not delete this line — it is what makes the pin effective rather than
# advisory. `$(HOME)/.local/bin` is beside it for `uv`, which installs there and which
# `make check` needs.
#
# Node is deliberately **not** here: it is nvm-managed on the machines this was written for
# and lives at a version-specific path no Makefile should hardcode. If `npm` is missing, the
# shell that sources nvm is what provides it (AGENTS.md, Toolchain).
export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/bin:$(PATH)

PROJECT  ?= $(HOME)/demo.escri
MANIFEST ?= tests/fixtures/manifest.json

# A prefix for the launch itself, empty by default. It exists because the webview a Tauri app
# renders in belongs to the operating system (ADR 0016 §5), and a machine whose WebKitGTK is
# not where the loader expects needs the binary wrapped rather than the recipe changed. The
# WSL2 machine this was written on has no `libwebkit2gtk-4.1-dev` and no `sudo`, so:
#
#   make run LAUNCH="unshare -Urm --propagation private $$HOME/.local/gtkdev/run-in-prefix.sh"
#
# On a machine with the packages installed it stays empty and nothing wraps anything.
LAUNCH ?=

.PHONY: help run dev webkit project check check-core check-app deps

# Named rather than left to position: make's default goal is the first target in the file, so
# adding a rule above `help` silently changes what a bare `make` does. It did, once.
.DEFAULT_GOAL := help

# Both windowed targets need WebKitGTK, and its absence arrives as
# "libwebkit2gtk-4.1.so.0: cannot open shared object file" from the loader — which names the
# library and not the package, and says nothing about what to type. One check, in front of the
# two targets that need it, replacing that with the apt line CI already uses. Skipped when
# LAUNCH is set, since a wrapper is how a machine without the packages supplies them.
webkit:
	@ldconfig -p 2>/dev/null | grep -q 'libwebkit2gtk-4\.1\.so\.0' || test -n '$(LAUNCH)' || { \
		echo 'WebKitGTK is not installed, so no window can open. Install it with:'; \
		echo; \
		echo '  sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \'; \
		echo '                      libayatana-appindicator3-dev libxdo-dev'; \
		echo; \
		echo 'That is the list .github/workflows/checks.yml installs, so it is known to work.'; \
		echo 'On a machine where you cannot install it, set LAUNCH to a wrapper that supplies'; \
		echo 'it — see the comment above LAUNCH in this file.'; \
		exit 1; }

help:
	@echo 'make run       build the frontend and open $$(PROJECT) in a window'
	@echo 'make dev       the same window against Vite, so a UI edit reloads'
	@echo 'make project   create $$(PROJECT) if it is not there yet'
	@echo 'make check     everything CLAUDE.md requires before a step is called done'
	@echo '              (check-core is the portable half; check-app needs WebKitGTK)'
	@echo 'make deps      reinstall the JavaScript dependencies'
	@echo
	@echo 'PROJECT  = $(PROJECT)'
	@echo 'MANIFEST = $(MANIFEST)'
	@echo 'LAUNCH   = $(LAUNCH)'

# `--features custom-protocol` is not optional and not a detail: without it the host builds in
# Tauri's dev mode, dials `devUrl`, and opens a window that says "Connection refused". With it
# the binary serves the `app/dist` it embedded, which is why the frontend is built first.
run: webkit project app/dist
	cargo build --release -p escribass-app --features custom-protocol
	$(LAUNCH) ./target/release/escribass-app --manifest $(MANIFEST) $(PROJECT)

# The iteration loop: Vite serves the frontend, the debug host loads it from there, and an edit
# to `app/src` reloads the window. No `--features custom-protocol` here — dev mode is exactly
# what dials the dev server.
#
# `ponytail:` Vite is started and killed by this recipe rather than run through
# `npm run tauri dev`, which would drive both but wants arguments passed through two layers of
# `--` to reach `--manifest`, and which was not what got tested. One shell, one trap, and the
# wait loop is there because the host does not retry a refused connection.
#
# The subshell `exec`s into Vite so that `$$!` is Vite itself. Backgrounding
# `npm --prefix app run dev` gives the pid of *npm*, and killing npm leaves the Vite it forked
# running on port 5173 for ever — which is what happened the first time this was tried. The
# binary and `app/package.json`'s `dev` script are the same command; the script is a one-word
# alias for it.
#
# **Close the window rather than pressing Ctrl-C.** Ctrl-C signals the whole process group, so
# Vite and the host both die — but the host dies without running destructors, and `.escri/lock`
# survives it. That is ADR 0012 §3 working as designed: an abnormal exit may be an exit taken
# mid-write, and a lock released then is worse than one left behind. The next launch says so
# and names the file to remove. Closing the window releases it cleanly, and this recipe ends
# with the window.
dev: webkit project app/node_modules
	@( cd app && exec ./node_modules/.bin/vite ) & \
	vite=$$!; \
	trap 'kill $$vite 2>/dev/null' EXIT INT TERM; \
	until curl -sf http://localhost:5173/ >/dev/null 2>&1; do sleep 0.3; done; \
	$(LAUNCH) cargo run -p escribass-app -- --manifest $(MANIFEST) $(PROJECT)

# A project to look at. `escribass-mcp --create` makes one and serves it, which is the whole
# of it — `app` has no File · New yet, and creating a project is not a tool (ADR 0006 §5).
#
# The `initialize` frame is not decoration: closing stdin on a server that has not completed
# the MCP handshake makes rmcp report a broken transport and exit non-zero, so a `< /dev/null`
# here created the project *and* failed the target. One frame in, EOF, clean exit.
project:
	@test -e $(PROJECT)/song.json || { \
		cargo build -q -p escribass-core --bin escribass-mcp && \
		printf '%s\n' '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"make","version":"1"}}}' \
		| ./target/debug/escribass-mcp --manifest $(MANIFEST) --author human \
			--create $(PROJECT) > /dev/null && \
		echo "created $(PROJECT)"; \
	}

# Split exactly where CI splits it, and for the same reason. `check-core` is the four checks
# CLAUDE.md requires and runs on any machine; `check-app` needs WebKitGTK's development
# packages, because building the Tauri host does. Running them as one target is right, and so
# is failing in a way that names which half — a developer changing `core` on a machine with no
# browser engine should get four green checks and one legible refusal, not a wall of
# pkg-config output from something called `check`.
check: check-core check-app

# `cargo test` and not `--workspace`: the workspace's default members are everything but the
# host (see `Cargo.toml`), which is what makes this half portable.
check-core: schema/node_modules
	./schema/codegen.sh --check
	./proto/codegen.sh --check
	cargo test
	cd schema && npx tsc --noEmit && node --import tsx --test tests/*.test.ts
	cd schema && uv run python -m unittest discover -s tests

# `--features custom-protocol` for the reason `run` uses it: that is the binary that ships,
# and it is the one that fails if `app/dist` is not there.
check-app: app/dist
	cd app && npx tsc --noEmit
	cargo test -p escribass-app --features custom-protocol

deps:
	npm --prefix schema ci
	npm --prefix app ci

# Directories as targets, so an install happens once and never again unless something is
# deleted. `schema` first and in full: `@escribass/schema` is a `file:` link, and node, Vite
# and `tsc` all resolve a linked package's own imports from the package's own directory — so
# `schema/gen/ts/song_pb.ts` looks for `@bufbuild/protobuf` in `schema/node_modules`, never in
# `app/node_modules`. The full install rather than CI's `--omit=dev`, because `make check`
# needs schema's `tsx` and `buf` as well.
schema/node_modules:
	npm --prefix schema ci

app/node_modules: schema/node_modules
	npm --prefix app ci

app/dist: app/node_modules $(wildcard app/src/*) app/index.html
	npm --prefix app run build
