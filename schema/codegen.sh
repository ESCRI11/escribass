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

for bin in buf uv protoc-gen-prost protoc-gen-prost-serde protoc-gen-es; do
  command -v "$bin" >/dev/null || { echo "missing codegen plugin: $bin" >&2; exit 1; }
done

# Hash the inputs and outputs so --check can tell "codegen would produce something else"
# apart from "you have uncommitted work" — git state is not the question being asked.
snapshot() {
  { find gen -type f -exec sha256sum {} + 2>/dev/null; sha256sum ./*.proto; } | sort -k2
}

before=""
[[ "${1:-}" == "--check" ]] && before="$(snapshot)"

buf format -w
buf lint

# These two trees are wholly generated, so stale files from a previous layout must go.
# gen/rust is not cleaned: its Cargo.toml, src/lib.rs and tests/ are hand-written.
rm -rf gen/python gen/ts

buf generate

if [[ "${1:-}" == "--check" ]]; then
  if [[ "$before" != "$(snapshot)" ]]; then
    echo "generated code or proto formatting is out of date — commit the result of schema/codegen.sh:" >&2
    diff <(printf '%s\n' "$before") <(snapshot) >&2 || true
    exit 1
  fi
fi
