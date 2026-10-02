//! `lock.baseline.json` against the files that really pin what it declares — **both ways**.
//!
//! CLAUDE.md #4 says every external dependency is pinned by commit in `lock.baseline.json`. Until
//! 2026-10-02 that was a claim with readers for two of its nine blocks: `engine` and
//! `bundled_plugins` were compared against the built engine's own report (`renders.rs`, and the
//! `Built from the pinned commits` step in CI), and `ai.provider` / `ai.model` against `core`'s
//! two constants (`core/tests/proposal.rs`). **Nothing read `app`, `schema`, `tool_api`, `dsp` or
//! `ci`**, nor `ai`'s four package pins. Review compared all of them by hand, found none
//! drifted, and said so: this is a missing invariant and was never a live bug. It is the kind
//! that stays missing until the day it matters, which is the day somebody bumps `react` in
//! `app/package.json` and the file that is supposed to say what this tree is built from quietly
//! does not.
//!
//! **The two directions, and why one is not enough.**
//!
//!   * **Declared but unpinned**: a row in `lock.baseline.json` whose source file does not say
//!     what it says. That is drift, in either direction, and [`PINS`] is the table that catches
//!     it — one row per declared pin, naming the file and the exact text that file must contain.
//!   * **Imported but undeclared**: a dependency a manifest declares that no row names. That is
//!     a dependency added without a pin, which is the thing CLAUDE.md #4 is actually about, and
//!     a table checked only in the first direction cannot see it: it is complete about what it
//!     lists and silent about what it does not.
//!
//! And a third assertion holds the first direction honest: **every leaf of
//! `lock.baseline.json` is classified**, as a pin checked here, a pin read somewhere named, a
//! pin with no source in the tree yet, or prose. A new block with no reader fails this file
//! rather than joining the seven that had none.
//!
//! **A line scan and not three parsers.** Every pin below is one literal line in its source —
//! `"react": "19.2.8"`, `version = "0.14.4"` under `name = "prost"`, `uses:
//! actions/checkout@v4` — and the workspace has a JSON parser and no TOML or YAML one. Adding
//! two parsers to read two lines each would be the dependency this file exists to notice
//! (CLAUDE.md #4). `ponytail:` if a pin ever needs real structure — a table whose shape matters,
//! a conditional in a workflow — that is the day to vendor a parser, and not before.

#[path = "common/mod.rs"]
mod common;

use common::workspace;
use serde_json::Value;
use std::collections::BTreeSet;

/// One declared pin: where it is in `lock.baseline.json`, what it pins, and the proof.
///
/// `name` is how the *source* spells the dependency — `pbjson-types`, not `pbjson_types` — and
/// is what the second direction matches a manifest's own line against. It is empty for a row
/// that is not a dependency at all: a toolchain channel, a runner, a compiler flag.
struct Pin {
    at: &'static str,
    name: &'static str,
    file: &'static str,
    needle: String,
}

fn pin(at: &'static str, name: &'static str, file: &'static str, needle: String) -> Pin {
    Pin { at, name, file, needle }
}

/// A package block in `Cargo.lock` or in a `uv.lock`, which are the same two lines in both.
fn locked(name: &str, version: &str) -> String {
    format!("name = \"{name}\"\nversion = \"{version}\"")
}

/// A dependency line in a `package.json`.
fn npm(name: &str, version: &str) -> String {
    format!("\"{name}\": \"{version}\"")
}

/// Every declared pin, the file that really pins it, and the text that file must contain.
fn pins(lock: &Value) -> Vec<Pin> {
    let at = |pointer: &str| -> String {
        lock.pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("lock.baseline.json has no string at `{pointer}`"))
            .to_string()
    };
    const CARGO: &str = "Cargo.lock";
    const CHECKS: &str = ".github/workflows/checks.yml";
    const APP_NPM: &str = "app/package.json";
    const SCHEMA_NPM: &str = "schema/package.json";

    let mut rows = vec![
        // ---- schema.rust: the codegen crates, resolved in the one Cargo.lock ----
        pin(
            "/schema/rust/toolchain/version",
            "",
            "rust-toolchain.toml",
            format!("channel = \"{}\"", at("/schema/rust/toolchain/version")),
        ),
        pin("/schema/rust/prost/version", "prost", CARGO, locked("prost", &at("/schema/rust/prost/version"))),
        pin("/schema/rust/pbjson/version", "pbjson", CARGO, locked("pbjson", &at("/schema/rust/pbjson/version"))),
        pin(
            "/schema/rust/pbjson_types/version",
            "pbjson-types",
            CARGO,
            locked("pbjson-types", &at("/schema/rust/pbjson_types/version")),
        ),
        pin("/schema/rust/serde/version", "serde", CARGO, locked("serde", &at("/schema/rust/serde/version"))),
        pin(
            "/schema/rust/serde_json/version",
            "serde_json",
            CARGO,
            locked("serde_json", &at("/schema/rust/serde_json/version")),
        ),
        // The three codegen plugins are `cargo install`ed by CI and are in no lock file, so the
        // workflow is their pin — which is also the one place a stale version would be invisible,
        // because a plugin already on the runner's cache is not reinstalled.
        pin(
            "/schema/rust/protoc_gen_prost/version",
            "",
            CHECKS,
            format!("protoc-gen-prost@{}", at("/schema/rust/protoc_gen_prost/version")),
        ),
        pin(
            "/schema/rust/protoc_gen_prost_serde/version",
            "",
            CHECKS,
            format!("protoc-gen-prost-serde@{}", at("/schema/rust/protoc_gen_prost_serde/version")),
        ),
        pin(
            "/tool_api/rust/protoc_gen_tonic/version",
            "",
            CHECKS,
            format!("protoc-gen-tonic@{}", at("/tool_api/rust/protoc_gen_tonic/version")),
        ),
        // ---- tool_api.rust ----
        pin("/tool_api/rust/tonic/version", "tonic", CARGO, locked("tonic", &at("/tool_api/rust/tonic/version"))),
        pin(
            "/tool_api/rust/tonic_prost/version",
            "tonic-prost",
            CARGO,
            locked("tonic-prost", &at("/tool_api/rust/tonic_prost/version")),
        ),
        pin("/tool_api/rust/rmcp/version", "rmcp", CARGO, locked("rmcp", &at("/tool_api/rust/rmcp/version"))),
        // Three are **caret ranges and not resolved versions**, so the manifest is their source
        // and `Cargo.lock` is not: `tokio = "1"` resolves to 1.53.1 today and the baseline
        // deliberately says `1`. A guard that compared these to the lock would fail on a pin
        // that is doing exactly what it says.
        pin(
            "/tool_api/rust/tokio/version",
            "tokio",
            "core/Cargo.toml",
            format!("tokio = {{ version = \"{}\"", at("/tool_api/rust/tokio/version")),
        ),
        pin(
            "/tool_api/rust/sha2/version",
            "sha2",
            "core/Cargo.toml",
            format!("sha2 = \"{}\"", at("/tool_api/rust/sha2/version")),
        ),
        pin(
            "/tool_api/rust/prost_types/version",
            "prost-types",
            "proto/Cargo.toml",
            format!("prost-types = \"{}\"", at("/tool_api/rust/prost_types/version")),
        ),
        // ---- app: one Rust crate, one build crate, and the npm set (ADR 0016 §2) ----
        pin("/app/tauri/version", "tauri", CARGO, locked("tauri", &at("/app/tauri/version"))),
        pin(
            "/app/tauri/build",
            "tauri-build",
            "app/src-tauri/Cargo.toml",
            format!("tauri-build = \"{}\"", at("/app/tauri/build")),
        ),
        pin("/app/tauri/cli", "@tauri-apps/cli", APP_NPM, npm("@tauri-apps/cli", &at("/app/tauri/cli"))),
        pin("/app/tauri/api", "@tauri-apps/api", APP_NPM, npm("@tauri-apps/api", &at("/app/tauri/api"))),
        pin("/app/react/version", "react", APP_NPM, npm("react", &at("/app/react/version"))),
        pin("/app/react_dom/version", "react-dom", APP_NPM, npm("react-dom", &at("/app/react_dom/version"))),
        pin("/app/types_react/version", "@types/react", APP_NPM, npm("@types/react", &at("/app/types_react/version"))),
        pin(
            "/app/types_react_dom/version",
            "@types/react-dom",
            APP_NPM,
            npm("@types/react-dom", &at("/app/types_react_dom/version")),
        ),
        pin("/app/vite/version", "vite", APP_NPM, npm("vite", &at("/app/vite/version"))),
        pin("/app/codemirror/version", "codemirror", APP_NPM, npm("codemirror", &at("/app/codemirror/version"))),
        pin(
            "/app/codemirror_lint/version",
            "@codemirror/lint",
            APP_NPM,
            npm("@codemirror/lint", &at("/app/codemirror_lint/version")),
        ),
        // The `_reused` rows are pins like any other — the block says only that `schema` pins
        // them too, not that nothing has to check them.
        pin("/app/_reused/typescript", "typescript", APP_NPM, npm("typescript", &at("/app/_reused/typescript"))),
        pin(
            "/app/_reused/bufbuild_protobuf",
            "@bufbuild/protobuf",
            APP_NPM,
            npm("@bufbuild/protobuf", &at("/app/_reused/bufbuild_protobuf")),
        ),
        pin("/app/_reused/tsx", "tsx", APP_NPM, npm("tsx", &at("/app/_reused/tsx"))),
        pin("/app/_reused/types_node", "@types/node", APP_NPM, npm("@types/node", &at("/app/_reused/types_node"))),
        // ---- schema.typescript ----
        pin("/schema/typescript/buf/version", "@bufbuild/buf", SCHEMA_NPM, npm("@bufbuild/buf", &at("/schema/typescript/buf/version"))),
        pin(
            "/schema/typescript/protoc_gen_es/version",
            "@bufbuild/protoc-gen-es",
            SCHEMA_NPM,
            npm("@bufbuild/protoc-gen-es", &at("/schema/typescript/protoc_gen_es/version")),
        ),
        pin(
            "/schema/typescript/bufbuild_protobuf/version",
            "@bufbuild/protobuf",
            SCHEMA_NPM,
            npm("@bufbuild/protobuf", &at("/schema/typescript/bufbuild_protobuf/version")),
        ),
        pin(
            "/schema/typescript/typescript/version",
            "typescript",
            SCHEMA_NPM,
            npm("typescript", &at("/schema/typescript/typescript/version")),
        ),
        pin("/schema/typescript/tsx/version", "tsx", SCHEMA_NPM, npm("tsx", &at("/schema/typescript/tsx/version"))),
        pin(
            "/schema/typescript/types_node/version",
            "@types/node",
            SCHEMA_NPM,
            npm("@types/node", &at("/schema/typescript/types_node/version")),
        ),
        // The service's TypeScript borrows schema's toolchain and installs none of its own
        // (`tool_api.typescript._note`), so schema's `package.json` is where these three are.
        pin(
            "/tool_api/typescript/_reused/protoc_gen_es",
            "@bufbuild/protoc-gen-es",
            SCHEMA_NPM,
            npm("@bufbuild/protoc-gen-es", &at("/tool_api/typescript/_reused/protoc_gen_es")),
        ),
        pin(
            "/tool_api/typescript/_reused/bufbuild_protobuf",
            "@bufbuild/protobuf",
            SCHEMA_NPM,
            npm("@bufbuild/protobuf", &at("/tool_api/typescript/_reused/bufbuild_protobuf")),
        ),
        pin(
            "/tool_api/typescript/_reused/typescript",
            "typescript",
            SCHEMA_NPM,
            npm("typescript", &at("/tool_api/typescript/_reused/typescript")),
        ),
        // ---- the Python environments: three locks, three interpreters ----
        pin("/ai/grpclib/version", "grpclib", "ai/uv.lock", locked("grpclib", &at("/ai/grpclib/version"))),
        pin("/ai/openai/version", "openai", "ai/uv.lock", locked("openai", &at("/ai/openai/version"))),
        pin("/ai/httpx2/version", "httpx2", "ai/uv.lock", locked("httpx2", &at("/ai/httpx2/version"))),
        pin(
            "/ai/build_backend/version",
            "uv_build",
            "ai/pyproject.toml",
            format!("requires = [\"{}\"]", at("/ai/build_backend/version")),
        ),
        pin(
            "/schema/python/betterproto2/version",
            "betterproto2",
            "schema/uv.lock",
            locked("betterproto2", &at("/schema/python/betterproto2/version")),
        ),
        pin(
            "/schema/python/betterproto2_compiler/version",
            "betterproto2-compiler",
            "schema/uv.lock",
            locked("betterproto2-compiler", &at("/schema/python/betterproto2_compiler/version")),
        ),
        pin(
            "/schema/python/pydantic/version",
            "pydantic",
            "schema/uv.lock",
            locked("pydantic", &at("/schema/python/pydantic/version")),
        ),
        pin(
            "/schema/python/grpclib/version",
            "grpclib",
            "schema/uv.lock",
            locked("grpclib", &at("/schema/python/grpclib/version")),
        ),
        pin(
            "/tool_api/python/_reused/betterproto2",
            "betterproto2",
            "schema/uv.lock",
            locked("betterproto2", &at("/tool_api/python/_reused/betterproto2")),
        ),
        pin(
            "/tool_api/python/_reused/betterproto2_compiler",
            "betterproto2-compiler",
            "schema/uv.lock",
            locked("betterproto2-compiler", &at("/tool_api/python/_reused/betterproto2_compiler")),
        ),
        pin(
            "/tool_api/python/_reused/pydantic",
            "pydantic",
            "schema/uv.lock",
            locked("pydantic", &at("/tool_api/python/_reused/pydantic")),
        ),
        // `.python-version` is a whole file, which is why `contains` is enough and exact: it has
        // one line in it.
        pin("/ai/python/version", "", "ai/.python-version", at("/ai/python/version")),
        // **The sibling review found**: the generative compiler's interpreter was compared to
        // the one it was running on and to nothing else, so a sidecar on 3.12.12 beside a
        // compiler on 3.13 would have been nobody's failure. `compilers/` has no block of its
        // own and needs none — it adds no external dependency (`compilers/generative/
        // pyproject.toml` says so) — but its interpreter is `ai`'s and this is where that is
        // written down.
        pin(
            "/ai/python/version",
            "",
            "compilers/generative/.python-version",
            at("/ai/python/version"),
        ),
        pin("/schema/python/python/version", "", "schema/.python-version", at("/schema/python/python/version")),
        pin(
            "/tool_api/python/_reused/python",
            "",
            "schema/.python-version",
            at("/tool_api/python/_reused/python"),
        ),
        // ---- ci ----
        pin(
            "/ci/actions_checkout/version",
            "actions/checkout",
            CHECKS,
            format!("actions/checkout@{}", at("/ci/actions_checkout/version")),
        ),
        pin(
            "/ci/actions_setup_node/version",
            "actions/setup-node",
            CHECKS,
            format!("actions/setup-node@{}", at("/ci/actions_setup_node/version")),
        ),
        pin(
            "/ci/actions_cache/version",
            "actions/cache",
            CHECKS,
            format!("actions/cache@{}", at("/ci/actions_cache/version")),
        ),
        pin(
            "/ci/actions_upload_artifact/version",
            "actions/upload-artifact",
            CHECKS,
            format!("actions/upload-artifact@{}", at("/ci/actions_upload_artifact/version")),
        ),
        pin(
            "/ci/actions_download_artifact/version",
            "actions/download-artifact",
            CHECKS,
            format!("actions/download-artifact@{}", at("/ci/actions_download_artifact/version")),
        ),
        pin("/ci/node/version", "", CHECKS, format!("node-version: \"{}\"", at("/ci/node/version"))),
        pin(
            "/ci/golden_render/runner",
            "",
            CHECKS,
            format!("runs-on: {}", at("/ci/golden_render/runner")),
        ),
        // The compiler's **major**, which is all the workflow names: `g++ 13.3` is what
        // ubuntu-24.04's `g++-13` happens to be, and the patch level is the runner's and not
        // this repository's to pin (ADR 0009 §1's "one toolchain, one machine shape").
        pin(
            "/ci/golden_render/cxx",
            "",
            CHECKS,
            format!(
                "CXX: g++-{}",
                at("/ci/golden_render/cxx")
                    .rsplit(' ')
                    .next()
                    .and_then(|version| version.split('.').next())
                    .expect("`g++ <major>.<minor>`")
            ),
        ),
        // The row's value is the flag list and then its history, so the needle is the leading
        // run of `-…` tokens: the flags are the pin and the sentence after them is prose. The
        // whole claim — these and no others, on every vendored source — is
        // `engine/cmake/check_flags.cmake`'s, which reads `compile_commands.json` at build time.
        pin(
            "/ci/golden_render/flags",
            "",
            "engine/CMakeLists.txt",
            at("/ci/golden_render/flags")
                .split_whitespace()
                .take_while(|token| token.starts_with('-'))
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ];
    rows.sort_by(|a, b| (a.at, &a.needle).cmp(&(b.at, &b.needle)));
    rows
}

/// Pins with a reader of their own, named, and what reads them.
///
/// These are the two blocks the review found already covered. Both are compared against the
/// **built engine's own report** rather than against a file in this tree, which is a stronger
/// check than anything here could be — and is why they are not repeated: a submodule's gitlink
/// cannot be read on a machine that has not checked the submodule out, and the `checks` job has
/// not.
const READ_ELSEWHERE: &[(&str, &str)] = &[
    ("/engine/tracktion_engine/commit", "tests/renders.rs and checks.yml's `Built from the pinned commits`"),
    ("/engine/juce/commit", "tests/renders.rs and checks.yml"),
    ("/engine/protobuf/commit", "tests/renders.rs and checks.yml"),
    ("/engine/grpc/commit", "tests/renders.rs and checks.yml"),
    ("/engine/rubberband/commit", "tests/renders.rs and checks.yml"),
    ("/bundled_plugins/surge_xt/commit", "tests/renders.rs and checks.yml"),
    ("/bundled_plugins/sfizz_ui/commit", "tests/renders.rs and checks.yml"),
    ("/bundled_plugins/dexed/commit", "tests/renders.rs and checks.yml"),
    ("/ai/provider", "core/tests/proposal.rs, against escribass_core::DEFAULT_PROVIDER"),
    ("/ai/model", "core/tests/proposal.rs, against escribass_core::DEFAULT_MODEL"),
];

/// Pins nothing in this tree can be compared against **yet**, and why each one.
///
/// Every row here is a promise about a milestone that has not happened or a fact that lives
/// outside the repository. None is an excuse: a row added here is a row a reviewer reads, and
/// the test below insists the list is exactly the leaves with no source — so a *new* pin cannot
/// quietly join it.
const NO_SOURCE_YET: &[(&str, &str)] = &[
    ("/engine/juce/version", "the commit is the pin; this names the tag it is, and nothing in the tree states it"),
    ("/engine/protobuf/version", "the commit is the pin; this names its tag"),
    ("/engine/grpc/version", "the commit is the pin; this names its tag"),
    ("/engine/rubberband/version", "the commit is the pin; this names its tag"),
    ("/engine/grpc/abseil/commit", "grpc's own third_party gitlink, two submodules deep; the engine reports grpc and not abseil"),
    ("/engine/onnxruntime/version", "M5's neural runtime (ADR 0003 §7). Nothing is vendored and nothing links it"),
    ("/engine/onnxruntime/commit", "M5's neural runtime; nothing is vendored"),
    ("/dsp/cmajor/version", "M5 (ADR 0023). Nothing in the tree vendors or builds it"),
    ("/dsp/cmajor/commit", "M5; nothing vendors it"),
    ("/dsp/clap_wrapper/version", "M5; nothing vendors it"),
    ("/dsp/clap_wrapper/commit", "M5; nothing vendors it"),
    ("/dsp/clap_sdk/version", "M5; nothing vendors it, and its commit is deliberately null"),
    ("/dsp/faust/version", "import-only, and M5 at the earliest"),
    ("/dsp/clap_sdk/commit", "null on purpose: clap-wrapper carries the SDK and this names the version it carries"),
    ("/dsp/faust/commit", "null on purpose: import-only, so there is nothing to build from a commit"),
    ("/bundled_plugins/surge_xt/version", "the commit is the pin; this names its tag"),
    ("/bundled_plugins/sfizz/version", "the commit is the pin; this names its tag"),
    ("/bundled_plugins/sfizz/commit", "the nested `library` gitlink inside sfizz-ui; the engine reports sfizz_ui, which is the one CI compares"),
    ("/bundled_plugins/sfizz_ui/version", "the commit is the pin; this names its tag"),
    ("/bundled_plugins/dexed/version", "the commit is the pin; this names its tag"),
    ("/bundled_plugins/airwindows/commit", "deferred with clap-wrapper (M4's row, now M5's): no submodule, and twelve characters rather than forty"),
    ("/schema/typescript/node/version", "the local Node, which no file in the tree states: there is no .nvmrc and no `engines`. `ci.node` pins the major CI runs"),
];

/// Keys whose value is prose, a date or a provenance note rather than a pin.
///
/// Matched on the **last** segment, so `/app/_reused/note` is prose and
/// `/app/_reused/typescript` is a pin. `_note` and `note` are the two spellings the file uses.
const PROSE: &[&str] = &[
    "_note", "note", "date", "tag", "repo", "ref", "via", "lts", "verified", "role",
    "model_note", "resolved_at", "schema_version", "notes",
];

fn read(file: &str) -> String {
    std::fs::read_to_string(workspace().join(file))
        .unwrap_or_else(|e| panic!("{file} is readable: {e}"))
}

fn baseline() -> Value {
    serde_json::from_str(&read("lock.baseline.json")).expect("lock.baseline.json is JSON")
}

/// Every leaf of `node` as a JSON pointer. An array is one leaf: nothing here holds a list of
/// pins, and `tokio.features` is a set compared whole.
fn leaves(node: &Value, path: String, found: &mut Vec<String>) {
    match node {
        Value::Object(fields) => {
            for (key, value) in fields {
                leaves(value, format!("{path}/{key}"), found);
            }
        }
        _ => found.push(path),
    }
}

#[test]
fn every_declared_pin_is_what_its_source_says() {
    // Direction one. Watched failing both ways: with `react` bumped to `19.2.9` in
    // `app/package.json` and with `/app/react/version` changed to `19.2.9` in the baseline —
    // the same assertion catches a dependency that moved and a pin that was edited to a lie,
    // because there is one text and two places it has to be.
    let lock = baseline();
    for Pin { at, file, needle, .. } in pins(&lock) {
        assert!(
            read(file).contains(&needle),
            "lock.baseline.json {at} pins `{needle}`, and {file} does not say so",
        );
    }
}

#[test]
fn tokios_feature_set_is_the_one_core_asks_for() {
    // The one array, and the only pin in the file that is not a version. A feature added to
    // `core`'s `tokio` line and not here is a dependency on something nobody pinned — `fs`,
    // `process`, `signal` — reached through a crate that is already in the tree.
    let lock = baseline();
    let declared: Vec<&str> = lock
        .pointer("/tool_api/rust/tokio/features")
        .and_then(Value::as_array)
        .expect("tokio.features is an array")
        .iter()
        .map(|feature| feature.as_str().expect("a feature is a string"))
        .collect();
    let needle = format!(
        "features = [{}]",
        declared.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ")
    );
    assert!(
        read("core/Cargo.toml").contains(&needle),
        "lock.baseline.json pins tokio {declared:?} and core/Cargo.toml asks for something else",
    );
}

#[test]
fn every_dependency_a_manifest_declares_is_declared_here_too() {
    // Direction two, which is the one CLAUDE.md #4 is actually about. The first direction is
    // complete about what the table lists and silent about what it does not, so it cannot
    // notice a dependency added with no pin — which is how `tauri-build`, `upload-artifact` and
    // `download-artifact` sat in the tree unpinned until 2026-10-02.
    //
    // Watched failing by adding `once_cell = "1"` to `core/Cargo.toml`, `"lodash": "4.17.21"` to
    // `app/package.json`, `"httpx==0.28"` to `ai/pyproject.toml` and
    // `uses: actions/stale@v9` to the workflow — one per ecosystem, each named and refused.
    let lock = baseline();
    let pinned = pins(&lock);
    let covered = |name: &str| pinned.iter().any(|pin| pin.name == name);

    // The repository's own crates, packages and modules, which are path and file links rather
    // than dependencies anything pins (`app/src-tauri/Cargo.toml` says so of `escribass-proto`).
    let ours = |name: &str| name.starts_with("escribass") || name.starts_with("@escribass/");

    let mut missing: Vec<String> = Vec::new();

    // Rust: every crate any manifest in the workspace asks for, by the line it asks on. A
    // `path =` line is one of ours, and a `[features]` or `[workspace]` line is not a
    // dependency at all — so the scan is bounded to the three dependency tables.
    for manifest in [
        "core/Cargo.toml",
        "proto/Cargo.toml",
        "schema/Cargo.toml",
        "tests/Cargo.toml",
        "app/src-tauri/Cargo.toml",
    ] {
        let text = read(manifest);
        let mut inside = false;
        for line in text.lines() {
            if let Some(header) = line.trim().strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
                inside = matches!(header, "dependencies" | "dev-dependencies" | "build-dependencies");
                // `[dependencies.foo]` is the same declaration written as a section, and a
                // scanner that only reads `foo = …` lines would not see it — a hole in a guard
                // whose whole job is not to have one. Nothing in this tree uses the form today.
                for table in ["dependencies.", "dev-dependencies.", "build-dependencies."] {
                    if let Some(name) = header.strip_prefix(table) {
                        if !ours(name) && !covered(name) {
                            missing.push(format!("{manifest} asks for the crate `{name}`"));
                        }
                    }
                }
                continue;
            }
            let Some((name, rest)) = line.split_once(" = ") else { continue };
            if !inside || line.starts_with(' ') || rest.contains("path =") || ours(name) {
                continue;
            }
            if !covered(name) {
                missing.push(format!("{manifest} asks for the crate `{name}`"));
            }
        }
    }

    // npm: `dependencies` and `devDependencies`, both read as JSON because there is a parser.
    for manifest in ["app/package.json", "schema/package.json", "proto/package.json"] {
        let text: Value = serde_json::from_str(&read(manifest)).expect("a package.json is JSON");
        for table in ["dependencies", "devDependencies"] {
            for name in text[table].as_object().map(|t| t.keys().collect()).unwrap_or(vec![]) {
                if !ours(name) && !covered(name) {
                    missing.push(format!("{manifest} asks for the npm package `{name}`"));
                }
            }
        }
    }

    // Python: the `dependencies` and `dependency-groups` lists, which are one requirement per
    // line in all four files. `compilers/generative/pyproject.toml` is in the list because its
    // own comment says this is where "M4 adds no external dependency" is checked.
    for manifest in [
        "ai/pyproject.toml",
        "compilers/generative/pyproject.toml",
        "schema/pyproject.toml",
        "proto/pyproject.toml",
    ] {
        for line in read(manifest).lines() {
            let line = line.trim();
            let Some(requirement) = line.strip_prefix('"') else { continue };
            let name = requirement
                .split(['"', '=', '[', '<', '>', ';'])
                .next()
                .expect("a requirement names something");
            if !name.is_empty() && !ours(name) && !covered(name) {
                missing.push(format!("{manifest} asks for the Python package `{name}`"));
            }
        }
    }

    // CI: every action any workflow uses.
    for line in read(".github/workflows/checks.yml").lines() {
        let Some((_, used)) = line.trim().split_once("uses: ") else { continue };
        let name = used.split('@').next().expect("an action is named").trim();
        if !covered(name) {
            missing.push(format!("the workflow uses the action `{name}`"));
        }
    }

    assert!(
        missing.is_empty(),
        "nothing in lock.baseline.json pins these (CLAUDE.md #4):\n  {}",
        missing.join("\n  "),
    );
}

#[test]
fn every_leaf_of_the_baseline_is_classified() {
    // What keeps the first direction honest. A table is complete about its own rows, so without
    // this a new block — `dsp` was one, `ci` was one — joins the file with nothing reading it,
    // which is exactly the state review found seven of nine blocks in.
    let lock = baseline();
    let mut found = Vec::new();
    leaves(&lock, String::new(), &mut found);

    let checked: BTreeSet<&str> = pins(&lock).iter().map(|pin| pin.at).collect();
    let elsewhere: BTreeSet<&str> = READ_ELSEWHERE.iter().map(|(at, _)| *at).collect();
    let no_source: BTreeSet<&str> = NO_SOURCE_YET.iter().map(|(at, _)| *at).collect();

    let unclassified: Vec<&String> = found
        .iter()
        .filter(|at| {
            let last = at.rsplit('/').next().expect("a pointer has a last segment");
            !PROSE.contains(&last)
                && !checked.contains(at.as_str())
                && !elsewhere.contains(at.as_str())
                && !no_source.contains(at.as_str())
                // The one array, which has a test of its own above.
                && at.as_str() != "/tool_api/rust/tokio/features"
        })
        .collect();
    assert!(
        unclassified.is_empty(),
        "these pins have no reader: add a row to `pins` if something in the tree pins them, or \
         to `NO_SOURCE_YET` with the reason nothing does:\n  {}",
        unclassified.iter().map(|at| at.as_str()).collect::<Vec<_>>().join("\n  "),
    );

    // And in the other direction: a row that no longer points at anything is a row somebody
    // forgot to delete, which is how a list of exceptions outlives the exceptions.
    for (at, why) in READ_ELSEWHERE.iter().chain(NO_SOURCE_YET) {
        assert!(
            found.iter().any(|leaf| leaf == at),
            "`{at}` is listed here ({why}) and is not in lock.baseline.json any more",
        );
    }
}

#[test]
fn the_three_python_environments_agree_about_their_interpreter() {
    // The second sibling review found. Three `uv.lock`s resolve the same requirements
    // separately, so a package in more than one of them can drift in one and not the others —
    // and one has: `multidict`, which `grpclib` brings and which nothing declares, is 6.9.1 in
    // `ai/uv.lock` and 7.0.0 in `compilers/generative/uv.lock`.
    //
    // The divergence is goldened rather than forbidden, because aligning it is a `uv lock` on a
    // machine with the network and this file cannot do one. What the assertion buys is that the
    // **next** one fails here: a transitive that drifts apart in two environments the same
    // generator runs through is the shape of "it works on my machine".
    //
    // Watched failing by moving `anyio` in one lock, which is reported beside `multidict`.
    let mut resolved: Vec<(String, std::collections::BTreeMap<String, String>)> = Vec::new();
    for lockfile in ["ai/uv.lock", "schema/uv.lock", "compilers/generative/uv.lock"] {
        let text = read(lockfile);
        let mut packages = std::collections::BTreeMap::new();
        let mut name: Option<String> = None;
        for line in text.lines() {
            if let Some(found) = line.strip_prefix("name = \"").and_then(|r| r.strip_suffix('"')) {
                name = Some(found.to_string());
            } else if let Some(version) =
                line.strip_prefix("version = \"").and_then(|r| r.strip_suffix('"'))
            {
                if let Some(held) = name.take() {
                    packages.insert(held, version.to_string());
                }
            }
        }
        assert!(packages.len() > 5, "{lockfile} parsed as {} packages", packages.len());
        resolved.push((lockfile.to_string(), packages));
    }

    let mut diverged: Vec<String> = Vec::new();
    for (index, (one, first)) in resolved.iter().enumerate() {
        for (other, second) in &resolved[index + 1..] {
            for (package, version) in first {
                if let Some(theirs) = second.get(package) {
                    if theirs != version {
                        diverged.push(format!("{package}: {version} in {one} vs {theirs} in {other}"));
                    }
                }
            }
        }
    }
    diverged.sort();
    assert_eq!(
        diverged,
        vec![
            "multidict: 6.9.1 in ai/uv.lock vs 7.0.0 in compilers/generative/uv.lock".to_string(),
            "multidict: 6.9.1 in schema/uv.lock vs 7.0.0 in compilers/generative/uv.lock".to_string(),
        ],
        "the Python environments drifted apart, or the one known divergence was fixed and this \
         list should shrink (docs/plan.md, known gaps)",
    );
}
