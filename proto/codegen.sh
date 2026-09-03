#!/usr/bin/env bash
# Regenerate the tool API types from song_tools.proto.
#
#   ./codegen.sh           regenerate in place
#   ./codegen.sh --check    regenerate, then fail if anything changed (CI gate)
#
# Separate from schema/codegen.sh because the two modules generate different things: the
# model goes to Rust, TypeScript and Python (§14.1), the service to Rust only until it has a
# consumer in either other language (ADR 0006 §7).
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PWD/../schema/node_modules/.bin:$PATH"

for bin in buf protoc-gen-prost protoc-gen-prost-serde; do
  command -v "$bin" >/dev/null || { echo "missing codegen plugin: $bin" >&2; exit 1; }
done

# Hash inputs and outputs so --check distinguishes "codegen would produce something else"
# from "you have uncommitted work" — git state is not the question being asked.
snapshot() {
  { find gen -type f -exec sha256sum {} + 2>/dev/null; sha256sum ./*.proto; } | sort -k2
}

before=""
[[ "${1:-}" == "--check" ]] && before="$(snapshot)"

buf format -w
buf lint

# gen/ is wholly generated, so stale files from a previous layout must go. Hand-written
# files live outside it: Cargo.toml, src/lib.rs, tests/.
rm -rf gen

buf generate

if [[ "${1:-}" == "--check" ]]; then
  if [[ "$before" != "$(snapshot)" ]]; then
    echo "generated code or proto formatting is out of date — commit the result of proto/codegen.sh:" >&2
    diff <(printf '%s\n' "$before") <(snapshot) >&2 || true
    exit 1
  fi
fi
