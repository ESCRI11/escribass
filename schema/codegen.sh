#!/usr/bin/env bash
# Regenerate model types for every language from the .proto files.
#
#   ./codegen.sh           regenerate in place
#   ./codegen.sh --check    regenerate, then fail if anything changed (CI gate)
#
# No language's model types are ever written by hand (docs/specs.md §4.1).
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PWD/node_modules/.bin:$PATH"

for bin in buf protoc-gen-prost protoc-gen-prost-serde protoc-gen-es; do
  command -v "$bin" >/dev/null || { echo "missing codegen plugin: $bin" >&2; exit 1; }
done

buf format -w
buf lint

# These two trees are wholly generated, so stale files from a previous layout must go.
# gen/rust is not cleaned: its Cargo.toml, src/lib.rs and tests/ are hand-written.
rm -rf gen/python gen/ts

buf generate

if [[ "${1:-}" == "--check" ]]; then
  git diff --exit-code -- gen/ \
    || { echo "generated code is out of date — run schema/codegen.sh and commit" >&2; exit 1; }
fi
