#!/usr/bin/env bash
# #38-r4: DIAGNOSTIC ONLY — this is NOT a completed compile gate.
#
# The full dep graph cannot finish on this host (no MSVC/MinGW C toolchain
# for ring/aws-lc-sys), so this script CANNOT prove a full windows build.
# It only checks that rustc begins type-checking our crate and that no
# non-dependency error appears before the C-toolchain failure. The real
# non-Linux compile gate is a CI windows job (tracked as a follow-up
# blackboard item, not claimed as locally proven here).
set -euo pipefail
cd "$(dirname "$0")"
TARGET="${1:-x86_64-pc-windows-msvc}"
OUT=$(cargo check --target "$TARGET" -p octoscode --no-default-features 2>&1 || true)
echo "$OUT" | grep -q "Compiling octoscode v0.3.0" || { echo "FAIL: octoscode never reached compilation"; echo "$OUT" | tail -5; exit 1; }
NONDEP=$(echo "$OUT" | grep -E "^error" | grep -vcE "ring|aws-lc-sys|custom build command" || true)
[ "$NONDEP" -eq 0 ] || { echo "FAIL: $NONDEP non-dependency error(s):"; echo "$OUT" | grep -E "^error" | grep -vE "ring|aws-lc-sys" | head; exit 1; }
echo "PASS: octoscode type-checked for $TARGET; only dep-C-toolchain failures remain"
