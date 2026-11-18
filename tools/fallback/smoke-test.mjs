#!/usr/bin/env node
// Smoke test for the JavaScript build: load it the way a page would, in plain
// Node, with WebAssembly taken away and no DOM.
//
// What should happen: the Rust `start` runs, finds no `window`, and panics. The
// panic hook then builds its message in Rust and hands it to console.error,
// which means the module was set up, memory works, and strings cross the
// boundary in both directions. Anything else (a syntax error in the translated
// module, an import that doesn't resolve, a read of `WebAssembly`) fails the build.
//
// usage: smoke-test.mjs <dir with huh_encrypter.js, huh_encrypter_bg.js, huh_encrypter_bg.wasm.js>
import { cpSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const dir = resolve(process.argv[2] ?? '');
const fail = (why) => {
    console.error(`fallback smoke test FAILED: ${why}`);
    process.exit(1);
};

// Behave like a browser with WebAssembly switched off, and notice any attempt to use it.
const touched = [];
if (!(delete globalThis.WebAssembly)) fail('could not remove globalThis.WebAssembly');
Object.defineProperty(globalThis, 'WebAssembly', {
    configurable: true,
    get() {
        touched.push(new Error('WebAssembly was read').stack);
        return undefined;
    },
});
if (typeof window !== 'undefined' || typeof document !== 'undefined') fail('this test expects no DOM');

// The files are ES modules; a scratch copy next to a package.json that says so
// lets every supported Node load them, whatever the project's own says.
const scratch = mkdtempSync(join(tmpdir(), 'huh-fallback-'));
let thrown = null;
const said = [];
const realError = console.error;
try {
    cpSync(dir, scratch, { recursive: true });
    writeFileSync(join(scratch, 'package.json'), '{ "type": "module" }\n');
    console.error = (...args) => said.push(args.map(String).join(' '));
    try {
        await import(pathToFileURL(join(scratch, 'huh_encrypter.js')).href);
    } catch (e) {
        thrown = e;
    }
} finally {
    console.error = realError;
    rmSync(scratch, { recursive: true, force: true });
}

if (touched.length) fail(`the JavaScript build touched WebAssembly:\n${touched[0]}`);
if (!thrown) fail('the app started without a DOM, which should be impossible');
if (thrown instanceof SyntaxError) fail(`the translated module does not parse: ${thrown.message}`);
const panic = said.find((line) => /panicked at .*\.rs:\d+:\d+/.test(line));
if (!panic) {
    fail(`expected a Rust panic about the missing DOM, got: ${thrown && thrown.stack ? thrown.stack : thrown}\n${said.join('\n')}`);
}
console.log(`fallback smoke test passed: Rust ran and stopped where it needs a page (${panic.split('\n')[0].trim()})`);
