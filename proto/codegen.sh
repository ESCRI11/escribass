#!/usr/bin/env bash
# Regenerate the tool API types from song_tools.proto.
#
#   ./codegen.sh           regenerate in place
#   ./codegen.sh --check    regenerate, then fail if anything changed (CI gate)
#
# Separate from schema/codegen.sh because the two modules generate different things: the
# model goes to Rust, TypeScript and Python (§14.1); the service goes to all three as well
# now, each language arriving in the milestone that gave it a consumer — TypeScript at M2 for
# `app`, Python at M3 for the sidecar that serves `Assistant` (ADR 0006 §7, ADR 0020 §1).
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PWD/../schema/node_modules/.bin:$PATH"

for bin in buf uv protoc-gen-prost protoc-gen-prost-serde protoc-gen-tonic protoc-gen-es; do
  command -v "$bin" >/dev/null || { echo "missing codegen plugin: $bin" >&2; exit 1; }
done

# Hash inputs and outputs so --check distinguishes "codegen would produce something else"
# from "you have uncommitted work" — git state is not the question being asked.
snapshot() {
  # __pycache__ is excluded for the reason schema/codegen.sh excludes it: anything that
  # imports the generated Python writes .pyc files under gen/python, and the rm -rf below
  # would turn them into false drift.
  { find gen -type f -not -path '*/__pycache__/*' -exec sha256sum {} + 2>/dev/null; sha256sum ./*.proto; } | sort -k2
}

before=""
[[ "${1:-}" == "--check" ]] && before="$(snapshot)"

buf format -w
buf lint

# gen/ is wholly generated, so stale files from a previous layout must go. Hand-written
# files live outside it: Cargo.toml, src/lib.rs, tests/.
rm -rf gen

buf generate

# The Python spelling of the `extern_path` and `rewrite_imports` options in buf.gen.yaml,
# done here because betterproto2 has neither. It generates a module for **every** file in the
# request — imports included, `file_to_generate` ignored — so the run above re-emitted
# song.proto and history.proto under gen/python, byte for byte identical to schema/'s copy
# and a different class at run time. A second `Song` in the tree imports and type-checks
# perfectly well and is wrong: the bar view is goldened against `escribass_schema`'s `Song`
# and would not take the one a `Prompt` arrives carrying (CLAUDE.md #1, ADR 0006 §4;
# ADR 0020 §1, amended 2026-09-22).
#
# google/ goes with them: it holds the well-known types, and song.proto's `Timestamp` was
# their only reader here.
pygen=gen/python/escribass_proto
rm -rf "$pygen/escribass/song" "$pygen/escribass/history" "$pygen/google"
sed -i \
  -e 's#^from \.\.\.song import v1 as #from escribass_schema.escribass.song import v1 as #' \
  -e 's#^from \.\.\.history import v1 as #from escribass_schema.escribass.history import v1 as #' \
  "$pygen"/escribass/*/v1/__init__.py

# The rewrite above is two regular expressions over generated text, so a compiler that
# changed how it spells a cross-package import would leave them matching nothing and this
# script would commit a tree that imports three deleted packages. Refuse instead: a silent
# no-op here is the failure mode this repository keeps finding.
if grep -rnE '^from \.\.\.(song|history) import|^from \.\.\.\.google import' "$pygen"; then
  echo "codegen.sh's import rewrite missed the lines above — the model's packages were deleted and something still names them relatively" >&2
  exit 1
fi
if ! grep -rq '^from escribass_schema\.escribass\.song import' "$pygen"; then
  echo "codegen.sh's import rewrite matched nothing — no generated module reaches the model at all" >&2
  exit 1
fi

# The descriptor set is what the MCP server turns into tool `inputSchema` (ADR 0006 §6).
# Generated from the same .proto as the Rust types, so a field cannot exist in one and not the
# other. Imports are included by default — the request messages reference song.proto, and a
# schema without those types would describe half a tool.
mkdir -p gen
buf build --as-file-descriptor-set -o gen/descriptor.binpb

if [[ "${1:-}" == "--check" ]]; then
  if [[ "$before" != "$(snapshot)" ]]; then
    echo "generated code or proto formatting is out of date — commit the result of proto/codegen.sh:" >&2
    diff <(printf '%s\n' "$before") <(snapshot) >&2 || true
    exit 1
  fi
fi
