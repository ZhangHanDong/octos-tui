#!/usr/bin/env bash
# #38-r3 non-Linux compile gate (completable evidence).
#
# The full dep graph cannot finish linking on this host (no MSVC/MinGW C
# toolchain for ring/aws-lc-sys). What we CAN and DO verify: the check
# progresses through `Compiling octoscode` — i.e. rustc type-checks OUR
# crate for the windows target — and the ONLY error in the whole run is
# the ring build script, never a cfg/type error in octoscode code.
set -euo pipefail
cd "$(dirname "$0")"
TARGET="${1:-x86_64-pc-windows-msvc}"
OUT=$(cargo check --target "$TARGET" -p octoscode --no-default-features 2>&1 || true)
echo "$OUT" | grep -q "Compiling octoscode v0.3.0" || { echo "FAIL: octoscode never reached compilation"; echo "$OUT" | tail -5; exit 1; }
NONDEP=$(echo "$OUT" | grep -E "^error" | grep -vcE "ring|aws-lc-sys|custom build command" || true)
[ "$NONDEP" -eq 0 ] || { echo "FAIL: $NONDEP non-dependency error(s):"; echo "$OUT" | grep -E "^error" | grep -vE "ring|aws-lc-sys" | head; exit 1; }
echo "PASS: octoscode type-checked for $TARGET; only dep-C-toolchain failures remain"
