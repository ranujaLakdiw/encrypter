# Encryptor
For Neja 💖
https://lakdiw.com/encrypter

- This uses Crystals Kyber encryption (a PQC algorithm) and AES algorithm to cipher texts. 
- Further developments and documentation will be made to make the process more user-friendly. 
- Files and images can be locked too: attach one (or drop it on the message box) and it becomes a `.huh` text file about 5× the size, which Decrypt turns back into the original file.
- Messages and files are compressed (LZMA2) before they are encrypted whenever that makes the huH shorter, and restored byte for byte on decrypt. huHs made before this still decrypt.
- Short notes are compressed against a built-in list of common English words and phrases, so even a one-line message usually comes out shorter. That list (`src/words.rs`) is part of the format and must never change.
- While it works, the page shows which step it is on: compressing then encrypting on the way in, decrypting then decompressing on the way out.

## Building

The page is written in Rust ([Leptos](https://leptos.dev)) and compiled to WebAssembly. `src/` is the source; `pkg/` is the built output that `index.html` loads, and it is committed so the site can be hosted as plain files.

`pkg/fallback/` is the same program translated to plain JavaScript (with Binaryen's `wasm2js`, see `tools/fallback/`). `index.html` loads it instead when the browser can't run WebAssembly: switched off (iOS Lockdown Mode, some locked-down setups) or too old. It is bigger and slower, and nothing else about it differs. Adding `?js` to the page's address runs it in any browser, to try it out.

After changing anything in `src/`:

```sh
./build.sh
```

That builds both. It needs Rust with the `wasm32-unknown-unknown` target, `wasm-pack` (`rustup target add wasm32-unknown-unknown`, `cargo install wasm-pack`), and Node with this project's packages (`npm install`, which brings in Binaryen).

## Deploying

The site is the files `tools/site.sh` gathers into `dist/`: `index.html`, `style_2.css`, `font/`, `img/` and `pkg/`. Nothing is compiled on the host, so a change under `src/` only goes live once `./build.sh` has been run and `pkg/` committed with it.

On Cloudflare Pages: framework preset None, build command `sh tools/site.sh`, build output directory `dist`.

## Testing

`cargo test` runs the crypto, huH codec and compression tests natively, including checks against messages, files and an ML-KEM exchange made by the original JavaScript page (`tests/vectors.json`) and against huHs in every compressed form made when each was introduced (`tests/squeezed.json`).
