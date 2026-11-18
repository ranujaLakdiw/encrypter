#!/usr/bin/env node
// Post-process wasm2js output so that growing memory DETACHES the old
// ArrayBuffer, the way a real WebAssembly.Memory does.
//
// Why this is needed (observed, binaryen 132 + wasm-bindgen 0.2.130):
//   wasm2js grows memory by allocating a new ArrayBuffer and copying. The old
//   buffer stays alive and keeps its length. wasm-bindgen's glue caches a
//   Uint8Array/DataView over `wasm.memory.buffer` and only re-creates them when
//   the cached view has become zero-length / detached:
//       if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0)
//   So after the first growth the glue keeps reading and writing the OLD copy
//   of memory (strings come out stale, strings passed in are lost).
//
// The patch inserts one statement into `__wasm_memory_grow`, right before
// `buffer = newBuffer;` (all HEAP views already point at the new buffer there):
//       wasm2js_detach(buffer);
// and adds the `wasm2js_detach` helper at the top of the file.
//
// It refuses to write anything unless the expected shape is found exactly once.
//
// usage: patch-wasm2js-memory-grow.mjs <wasm2js-output.js> [<out.js>]
import { readFileSync, writeFileSync } from 'node:fs';

const [inPath, outPath = inPath] = process.argv.slice(2);
if (!inPath) {
  console.error('usage: patch-wasm2js-memory-grow.mjs <wasm2js-output.js> [<out.js>]');
  process.exit(2);
}
let src = readFileSync(inPath, 'utf8');
const fail = (msg) => {
  console.error(`patch-wasm2js-memory-grow: ${msg}\n  (wasm2js output shape changed? file: ${inPath})`);
  process.exit(1);
};

if (src.includes('wasm2js_detach')) fail('file is already patched');

// 1. Locate the grow function.
const growStarts = [...src.matchAll(/function __wasm_memory_grow\s*\(\s*pagesToAdd\s*\)\s*\{/g)];
if (growStarts.length !== 1) fail(`expected exactly 1 __wasm_memory_grow(pagesToAdd), found ${growStarts.length}`);
const growStart = growStarts[0].index;

// 2. Inside it (before the next top-of-factory `return {`), find `buffer = newBuffer;`.
const growEnd = src.indexOf('return oldPages;', growStart);
if (growEnd < 0) fail('could not find `return oldPages;` after __wasm_memory_grow');
const body = src.slice(growStart, growEnd);
const assigns = [...body.matchAll(/(^|\n)([ \t]*)buffer = newBuffer;/g)];
if (assigns.length !== 1) fail(`expected exactly 1 \`buffer = newBuffer;\` in __wasm_memory_grow, found ${assigns.length}`);
// Sanity: the copy into the new buffer and the view re-creation must precede it.
const assignAt = assigns[0].index + assigns[0][1].length;
const before = body.slice(0, assignAt);
if (!/new ArrayBuffer\(/.test(before)) fail('no `new ArrayBuffer(` before `buffer = newBuffer;`');
if (!/\.set\(HEAP8\)/.test(before)) fail('no `.set(HEAP8)` copy before `buffer = newBuffer;`');
if (!/HEAPU8 = new Uint8Array\(newBuffer\)/.test(before)) fail('HEAPU8 is not re-created on newBuffer before `buffer = newBuffer;`');

const indent = assigns[0][2];
const absolute = growStart + assignAt;
src = src.slice(0, absolute) + `${indent}wasm2js_detach(buffer);\n` + src.slice(absolute);

// 3. Add the helper. It goes after the leading `import` lines (ES-module output).
const helper = `
// --- added by patch-wasm2js-memory-grow.mjs ---------------------------------
// Detach a memory buffer that has just been replaced by a bigger copy, so that
// every typed array / DataView still pointing at it becomes zero-length, as
// happens with a real WebAssembly.Memory. wasm-bindgen's glue depends on that.
function wasm2js_detach(old) {
  try {
    if (typeof old.transfer === 'function') { old.transfer(0); return; }           // ES2024
    if (typeof structuredClone === 'function') { structuredClone(old, { transfer: [old] }); return; }
    var ch = new MessageChannel();                                                  // oldest route
    ch.port1.postMessage(old, [old]);
    ch.port1.close();
    ch.port2.close();
  } catch (e) {
    throw new Error('wasm2js fallback: could not detach the old memory buffer after growth: ' + e);
  }
}
// -----------------------------------------------------------------------------
`;
const importBlock = /^(?:\s*import [^\n]*\n)*/.exec(src)[0];
src = src.slice(0, importBlock.length) + helper + src.slice(importBlock.length);

writeFileSync(outPath, src);
console.log(`patch-wasm2js-memory-grow: patched __wasm_memory_grow in ${outPath} (+${helper.length + indent.length + 24} chars)`);
