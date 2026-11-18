#!/usr/bin/env node
// Remove named entries from a wasm module's `target_features` custom section.
// Every other byte of the module is copied through unchanged (no re-encoding
// of code, so wasm-bindgen's descriptor interpreter sees the exact same
// functions that cargo produced).
//
// Why: wasm-bindgen (0.2.130 observed) switches on its externref transform when
// it finds `+reference-types` in `target_features`, and its multi-value
// transform when it finds `+multivalue`. Rust enables both target features by
// default, but rustc does not emit any reference-typed or multi-value code for
// this crate, so dropping the two entries is a pure metadata change.
//
// usage: strip-target-features.mjs <in.wasm> <out.wasm> <feature> [<feature>...]
import { readFileSync, writeFileSync } from 'node:fs';

const [inPath, outPath, ...remove] = process.argv.slice(2);
if (!inPath || !outPath || remove.length === 0) {
  console.error('usage: strip-target-features.mjs <in.wasm> <out.wasm> <feature> [<feature>...]');
  process.exit(2);
}

const src = readFileSync(inPath);
if (src.readUInt32LE(0) !== 0x6d736100 || src.readUInt32LE(4) !== 1) {
  console.error('not a wasm v1 binary: ' + inPath);
  process.exit(1);
}

function readLeb(buf, pos) {
  let result = 0, shift = 0, b;
  do {
    b = buf[pos++];
    result += (b & 0x7f) * 2 ** shift;
    shift += 7;
  } while (b & 0x80);
  return [result, pos];
}
function leb(n) {
  const out = [];
  do {
    let b = n & 0x7f;
    n = Math.floor(n / 128);
    if (n !== 0) b |= 0x80;
    out.push(b);
  } while (n !== 0);
  return Buffer.from(out);
}

const chunks = [src.subarray(0, 8)];
let pos = 8;
let found = false;
const removed = [];
let before = [], after = [];

while (pos < src.length) {
  const start = pos;
  const id = src[pos++];
  let size; [size, pos] = readLeb(src, pos);
  const bodyStart = pos;
  const bodyEnd = pos + size;
  pos = bodyEnd;

  if (id === 0) {
    let [nameLen, p] = readLeb(src, bodyStart);
    const name = src.subarray(p, p + nameLen).toString('utf8');
    if (name === 'target_features') {
      found = true;
      p += nameLen;
      let count; [count, p] = readLeb(src, p);
      const kept = [];
      for (let i = 0; i < count; i++) {
        const entryStart = p;
        const prefix = String.fromCharCode(src[p++]);
        let len; [len, p] = readLeb(src, p);
        const feature = src.subarray(p, p + len).toString('utf8');
        p += len;
        before.push(prefix + feature);
        if (remove.includes(feature)) {
          removed.push(prefix + feature);
        } else {
          kept.push(src.subarray(entryStart, p));
          after.push(prefix + feature);
        }
      }
      if (p !== bodyEnd) throw new Error('target_features: trailing bytes');
      const nameBytes = Buffer.from('target_features', 'utf8');
      const body = Buffer.concat([leb(nameBytes.length), nameBytes, leb(kept.length), ...kept]);
      chunks.push(Buffer.from([0]), leb(body.length), body);
      continue;
    }
  }
  chunks.push(src.subarray(start, bodyEnd));
}

if (!found) {
  console.error('error: no target_features custom section in ' + inPath);
  process.exit(1);
}
const missing = remove.filter((f) => !removed.some((r) => r.slice(1) === f));
const out = Buffer.concat(chunks);
writeFileSync(outPath, out);
console.log(`target_features before: ${before.join(' ')}`);
console.log(`target_features after : ${after.join(' ')}`);
console.log(`removed: ${removed.join(' ') || '(none)'}${missing.length ? `   not present: ${missing.join(' ')}` : ''}`);
console.log(`${inPath}: ${src.length} bytes -> ${outPath}: ${out.length} bytes`);
