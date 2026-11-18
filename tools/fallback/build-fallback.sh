#!/bin/bash
# Builds the page a second time as plain JavaScript, for browsers where
# WebAssembly is switched off (iOS Lockdown Mode, some locked-down setups) or too
# old for what Rust emits. It is the same Rust program: the compiled module is
# translated to JavaScript by Binaryen's wasm2js. index.html loads it only when
# the WebAssembly build can't run.
#
#   build-fallback.sh <raw.wasm> <out-dir>
#
#   <raw.wasm>  target/wasm32-unknown-unknown/release/huh_encrypter.wasm, i.e. the
#               module as cargo made it, BEFORE wasm-bindgen ran.
#   <out-dir>   receives three files (ES modules; importing the first starts the page):
#                 huh_encrypter.js          entry: wires glue + module, starts the app
#                 huh_encrypter_bg.js       wasm-bindgen glue (bundler target, no WebAssembly)
#                 huh_encrypter_bg.wasm.js  the module, translated by wasm2js
#
# Needs: node, the `binaryen` npm package (`npm install` in the project root), and
# a wasm-bindgen CLI of exactly the version in Cargo.lock (wasm-pack downloads one
# into its cache on every build, which is where this looks first).
#
# Worked out against wasm-bindgen 0.2.130 and Binaryen 132. Every assumption about
# their output is checked below, so a newer version that breaks one stops the
# build with a message instead of producing a broken page.
#
# Environment overrides:
#   WASM_BINDGEN   wasm-bindgen CLI to use (still has to match Cargo.lock)
#   BINARYEN_BIN   directory holding wasm-opt, wasm-merge, wasm-as, wasm-dis, wasm2js
#   OUT_NAME       base name of the outputs            (default: huh_encrypter)
#   WASM_OPT_LEVEL optimisation flag for wasm-opt      (default: -O;  "none" to skip)
#   WASM_OPT_EXTRA extra wasm-opt passes, space separated, run before the level flag
#   WASM2JS_LEVEL  optimisation flag for wasm2js       (default: -O;  "none" to skip)
#   WORK_DIR       where intermediate files go         (default: <out-dir>/.work)
#   KEEP_WORK=1    keep WORK_DIR afterwards
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RAW="${1:?usage: build-fallback.sh <raw.wasm> <out-dir>}"
OUT="${2:?usage: build-fallback.sh <raw.wasm> <out-dir>}"
BINARYEN_BIN="${BINARYEN_BIN:-$ROOT/node_modules/.bin}"
OUT_NAME="${OUT_NAME:-huh_encrypter}"
WASM_OPT_LEVEL="${WASM_OPT_LEVEL:--O}"
WASM2JS_LEVEL="${WASM2JS_LEVEL:--O}"
WORK="${WORK_DIR:-$OUT/.work}"

for tool in wasm-opt wasm-merge wasm-as wasm-dis wasm2js; do
  [ -x "$BINARYEN_BIN/$tool" ] || { echo "error: $BINARYEN_BIN/$tool not found. Run \`npm install\` in $ROOT first (it installs binaryen)." >&2; exit 1; }
done

# The glue wasm-bindgen writes has to match the wasm-bindgen crate the page was
# compiled with, to the patch version.
WANT="$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[^0-9.]/, ""); print; exit }' "$ROOT/Cargo.lock")"
[ -n "$WANT" ] || { echo "error: could not read the wasm-bindgen version from $ROOT/Cargo.lock" >&2; exit 1; }
FOUND=""
for candidate in "${WASM_BINDGEN:-}" "$HOME"/Library/Caches/.wasm-pack/wasm-bindgen-*/wasm-bindgen "$HOME"/.cache/.wasm-pack/wasm-bindgen-*/wasm-bindgen "$(command -v wasm-bindgen || true)"; do
  if [ -n "$candidate" ] && [ -x "$candidate" ] && [ "$("$candidate" --version 2>/dev/null)" = "wasm-bindgen $WANT" ]; then
    FOUND="$candidate"
    break
  fi
done
[ -n "$FOUND" ] || { echo "error: no wasm-bindgen $WANT CLI found (looked in wasm-pack's cache and on PATH). \`cargo install wasm-bindgen-cli --version $WANT\` provides one." >&2; exit 1; }
WASM_BINDGEN="$FOUND"

# First JS-heap index that is not a reserved slot; see externref-xform-stub.wat.
STATICS_END=1028

now_ms() { node -e 'console.log(Date.now())'; }
T0=$(now_ms)
step() { # step <label> <cmd...>   (prints the command and how long it took)
  local label="$1"; shift
  local t=$(now_ms)
  echo "[$label] \$ $*"
  "$@"
  echo "[$label] done in $(( $(now_ms) - t )) ms"
}

mkdir -p "$OUT" "$WORK"
rm -rf "$WORK/bindgen"

# --- 1. turn off wasm-bindgen's externref + multi-value transforms -----------
# wasm-bindgen has no switch for this (WASM_BINDGEN_EXTERNREF=0 and friends do
# nothing); it keys off the module's `target_features` section. Removing the two
# entries is a metadata-only edit as long as rustc emitted no reference-typed or
# multi-value code, which step 1b verifies.
step 1-strip-features node "$HERE/strip-target-features.mjs" "$RAW" "$WORK/raw.noref.wasm" reference-types multivalue
# 1b. Binaryen takes its feature set from the (now shorter) target_features
# section and validates the module against it: any externref / table.get /
# multi-value signature in the code would be rejected here.
step 1b-validate "$BINARYEN_BIN/wasm-opt" "$WORK/raw.noref.wasm" -o "$WORK/raw.noref.validated.wasm"
rm -f "$WORK/raw.noref.validated.wasm"

# --- 2. wasm-bindgen, bundler target ----------------------------------------
# The bundler target is the one wasm-bindgen documents for wasm2js: its glue
# never mentions `WebAssembly`; it just imports "./<name>_bg.wasm" as an ES module.
step 2-wasm-bindgen "$WASM_BINDGEN" "$WORK/raw.noref.wasm" --target bundler --no-typescript --out-dir "$WORK/bindgen" --out-name "$OUT_NAME"

GLUE_ENTRY="$WORK/bindgen/$OUT_NAME.js"
GLUE_BG="$WORK/bindgen/${OUT_NAME}_bg.js"
BG_WASM="$WORK/bindgen/${OUT_NAME}_bg.wasm"

# The glue must be the heap-object flavour, not the externref-table flavour.
if grep -q '__wbindgen_externrefs\|__externref_table_alloc' "$GLUE_BG"; then
  echo "error: wasm-bindgen still produced externref glue (target_features strip had no effect?)" >&2; exit 1
fi
grep -q 'function addHeapObject' "$GLUE_BG" || { echo "error: glue has no addHeapObject(); unexpected wasm-bindgen output" >&2; exit 1; }
if grep -q 'WebAssembly' "$GLUE_BG" "$GLUE_ENTRY"; then
  echo "error: bundler glue references WebAssembly; the fallback would not be WebAssembly-free" >&2; exit 1
fi

# --- 3. resolve the import wasm-bindgen 0.2.130 leaves dangling --------------
# With the externref transform off, `__wbindgen_externref_xform__.__wbindgen_externref_statics_end`
# stays an import that nothing provides. Link in a one-function module for it.
"$BINARYEN_BIN/wasm-dis" "$BG_WASM" -o "$WORK/bindgen.wat"
XFORM_IMPORTS="$(grep -E '^\s*\(import "__wbindgen_externref_xform__"' "$WORK/bindgen.wat" | grep -oE '"__wbindgen_externref_xform__" "[^"]*"' || true)"
if [ -z "$XFORM_IMPORTS" ]; then
  echo "[3-merge] no __wbindgen_externref_xform__ imports left by wasm-bindgen; nothing to resolve"
  cp "$BG_WASM" "$WORK/merged.wasm"
elif [ "$XFORM_IMPORTS" = '"__wbindgen_externref_xform__" "__wbindgen_externref_statics_end"' ]; then
  # The constant has to agree with the glue's own idea of the reserved slots.
  GLUE_RESERVED="$(grep -A1 '^function dropObject(idx)' "$GLUE_BG" | grep -oE 'idx < [0-9]+' | grep -oE '[0-9]+' || true)"
  if [ "$GLUE_RESERVED" != "$STATICS_END" ]; then
    echo "error: glue's dropObject() reserves '$GLUE_RESERVED' slots but the stub returns $STATICS_END; update externref-xform-stub.wat and STATICS_END" >&2; exit 1
  fi
  grep -q "(i32.const $STATICS_END)" "$HERE/externref-xform-stub.wat" || { echo "error: stub .wat does not return $STATICS_END" >&2; exit 1; }
  step 3a-wasm-as "$BINARYEN_BIN/wasm-as" "$HERE/externref-xform-stub.wat" -o "$WORK/xform-stub.wasm"
  step 3b-wasm-merge "$BINARYEN_BIN/wasm-merge" "$BG_WASM" main "$WORK/xform-stub.wasm" __wbindgen_externref_xform__ -o "$WORK/merged.wasm"
else
  echo "error: unexpected __wbindgen_externref_xform__ imports after wasm-bindgen:" >&2
  echo "$XFORM_IMPORTS" >&2; exit 1
fi

# --- 4. lower what wasm2js cannot translate, then optimise -------------------
# Needed: --llvm-nontrapping-fptoint-lowering (wasm2js fails on i64.trunc_sat_*).
# NOT lowered on purpose: memory.copy/fill (wasm2js maps them to
# TypedArray.copyWithin/fill, far faster than a byte loop) and sign-extension
# ops (wasm2js translates them directly).
OPT_ARGS=("$WORK/merged.wasm" --llvm-nontrapping-fptoint-lowering)
# shellcheck disable=SC2206   (WASM_OPT_EXTRA is a deliberate word-split list of extra passes)
[ -n "${WASM_OPT_EXTRA:-}" ] && OPT_ARGS+=($WASM_OPT_EXTRA)
[ "$WASM_OPT_LEVEL" != "none" ] && OPT_ARGS+=("$WASM_OPT_LEVEL")
step 4-wasm-opt "$BINARYEN_BIN/wasm-opt" "${OPT_ARGS[@]}" -o "$WORK/lowered.wasm"

# --- 5. wasm -> JavaScript ----------------------------------------------------
JS_ARGS=("$WORK/lowered.wasm")
[ "$WASM2JS_LEVEL" != "none" ] && JS_ARGS+=("$WASM2JS_LEVEL")
step 5-wasm2js "$BINARYEN_BIN/wasm2js" "${JS_ARGS[@]}" -o "$WORK/module.js"

# The loader relies on these properties of the wasm2js output; stop if they change.
IMPORT_LINES="$(grep -E '^import ' "$WORK/module.js" || true)"
EXPECTED_IMPORT="import * as \$_${OUT_NAME}_bg_js from './${OUT_NAME}_bg.js';"
if [ "$IMPORT_LINES" != "$EXPECTED_IMPORT" ]; then
  echo "error: wasm2js output should have exactly this one import line:" >&2
  echo "  $EXPECTED_IMPORT" >&2
  echo "but has:" >&2; echo "$IMPORT_LINES" >&2; exit 1
fi
for name in memory __wbindgen_start __wbindgen_malloc __wbindgen_realloc; do
  grep -q "^export var $name = retasmFunc\.$name;" "$WORK/module.js" || { echo "error: wasm2js output does not export '$name' the expected way" >&2; exit 1; }
done
if grep -q 'WebAssembly' "$WORK/module.js"; then
  echo "error: wasm2js output mentions WebAssembly" >&2; exit 1
fi

# --- 6. make memory growth detach the old buffer (glue depends on it) ---------
step 6-patch-grow node "$HERE/patch-wasm2js-memory-grow.mjs" "$WORK/module.js" "$WORK/module.patched.js"

# --- 7. assemble the output directory -----------------------------------------
# Entry: wasm-bindgen's own bundler entry, with the one .wasm import pointed at
# the translated module.
ENTRY_HITS="$(grep -c "from \"./${OUT_NAME}_bg.wasm\";" "$GLUE_ENTRY" || true)"
if [ "$ENTRY_HITS" != "1" ]; then
  echo "error: expected exactly one import of ./${OUT_NAME}_bg.wasm in the bundler entry, found $ENTRY_HITS" >&2; exit 1
fi
sed "s|from \"./${OUT_NAME}_bg.wasm\";|from \"./${OUT_NAME}_bg.wasm.js\";|" "$GLUE_ENTRY" > "$OUT/$OUT_NAME.js"
cp "$GLUE_BG" "$OUT/${OUT_NAME}_bg.js"
cp "$WORK/module.patched.js" "$OUT/${OUT_NAME}_bg.wasm.js"

echo
echo "fallback written to $OUT ($(( $(now_ms) - T0 )) ms total):"
for f in "$OUT_NAME.js" "${OUT_NAME}_bg.js" "${OUT_NAME}_bg.wasm.js"; do
  printf '  %-32s %9d bytes  %8d gzip -9\n' "$f" "$(wc -c < "$OUT/$f")" "$(gzip -9 -c "$OUT/$f" | wc -c)"
done
[ "${KEEP_WORK:-0}" = "1" ] || [ -n "${WORK_DIR:-}" ] || rm -rf "$WORK"
