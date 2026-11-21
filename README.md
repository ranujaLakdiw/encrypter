# Encryptor
For Neja 💖
https://lakdiw.com/encrypter

- This uses ML-KEM-1024 (Crystals Kyber, a post-quantum algorithm) to agree on a key and AES-256-GCM to cipher texts.
- Further developments and documentation will be made to make the process more user-friendly. 
- Files and images can be locked too: attach one (or drop it on the message box) and it becomes a `.huh` text file about 5× the size, which Decrypt turns back into the original file.
- Messages and files are compressed (LZMA2) before they are encrypted whenever that makes the huH shorter, and restored byte for byte on decrypt. huHs made before this still decrypt.
- Short notes are compressed against a built-in list of common English words and phrases, so even a one-line message usually comes out shorter. That list (`src/words.rs`) is part of the format and must never change.
- While it works, the page shows which step it is on: compressing then encrypting on the way in, decrypting then decompressing on the way out.
- A paired session can be remembered on the device, so it doesn't have to be paired again next visit: give it a nickname and a passphrase. What is saved (in the browser's local storage, nowhere else) is the session key sealed with AES-256-GCM under a key stretched from the passphrase with PBKDF2-HMAC-SHA256, 600,000 rounds. The passphrase itself is never stored, so a forgotten one can't be recovered: forget that session and pair again. Open a remembered session from the Session tab with its passphrase; ending the session locks it away again. The sealed format (`src/keep.rs`) must keep opening what people have saved, so it must not change either.

## How a huH is locked

Every huH is sealed with AES-256-GCM under a key that is made for that one huH and never used again (`src/envelope.rs`):

- in a session: `0x02 · salt (16 bytes) · the payload, padded and encrypted · tag (16 bytes)`
- with a password: `0x01 · salt (16 bytes) · key check (8 bytes) · the payload, padded and encrypted · tag (16 bytes)`

**In a session**, the key is HKDF-SHA256 of the session's secret and the salt.

**With a password**, the password is first stretched with PBKDF2-HMAC-SHA256, 600,000 rounds, and the page won't lock with one shorter than 8 characters. The rounds are what make guessing slow: a laptop that could try twenty million passwords a second against an old huH manages a couple of hundred against a new one. They can't make a short or common password a good one, so the password is still the weakest part of a password huH. The key check ties a huH to the password it was locked with: it comes out of the same stretching as the key, and a huH opens only under a password whose check is the one it carries. Without it a single forged huH could be made to pass under thousands of guesses at once, and anyone who would try it and say what happened would be testing all of them with one paste.

**The tag** is a check on every byte the huH spells, head included. Whoever doesn't hold the key can't change one and have it still open, and can't make a huH of their own that opens.

**The padding** fills the payload out to the next whole 16 bytes, so that a huH's length says little about what is in it ("yes" and "no" come out the same length). It is also why new huHs are longer than old ones: a short note is about half as long again.

What a huH does not say is who made it or when. Both people in a session hold the same secret, and a password is a password, so a huH posted a second time (your own, handed back to you, or last week's answer to this week's question) opens like any other. Nothing here can tell.

huHs made before this was the format (AES-256-CBC, with a key made by one round of MD5 and nothing to notice a change) still open, and what comes out of one is marked "old huH, unchecked": it says what it says, but it can't be vouched for. Nothing is locked that way any more, which also means both people need the page as it is now. A tab left open from before can't read a new huH until it is reloaded.

Old huHs are also why **a password that was used before this should not be used again**. Anyone holding a single old huH can still try twenty million guesses a second against it, and a password found that way opens the new huHs too.

This layout is what people's huHs are written in from now on, so, like the word list and the sealed sessions, it must not change. `tests/enveloped.json` holds huHs that have to keep opening.

## Pairing a session

Each person makes an ML-KEM-1024 keypair and sends the public key; each replies to the other's key; each opens the reply they are sent. The session's secret is HKDF-SHA256 over the two secrets inside those replies *and* a hash of all four codes that were swapped (`src/kem.rs`), so both people get the same secret only if both saw the very same codes. Your own key or reply pasted back where the other person's belongs is refused, and so is a key pasted where a reply belongs.

Swapping codes keeps the secret from anyone who only listens. It can't tell whose codes arrived: someone able to change them on the way could hand each person their own and sit in the middle. That is what the **check number** in the session card is for. It is forty digits cut from the secret, the same for both people only if they are in the same session with nobody between them.

Two things make it worth anything:

- **All of it** has to be compared. Whoever sits in the middle sends her replies last and can make as many as she likes first, so she can arrange for part of the number to agree: the first ten digits take about a second. Getting all forty to agree would take some 2^66 fresh ML-KEM replies, which nobody has.
- It has to be compared **somewhere the codes did not travel** (on a call, face to face), because whoever could change the codes could change a number sent the same way.

The short name (amber-arc-07) is for telling sessions apart and catching a code that was pasted wrong. There are forty thousand of them, and someone in the middle can make two match in no time, so a matching name is not proof of anything.

Sessions that were paired or remembered before this still work (what is remembered is the secret itself), and show a check number of their own now. They were paired the old way, though, with nothing to compare: compare the number, or pair again.

A session has one secret for as long as it lasts. Anyone who gets hold of it (from a remembered session with a weak passphrase, say) can read everything that was ever locked in that session. Ending it and pairing again starts a new one.

## What the page loads

Only its own files. The fonts (`font/`) and the backdrop script (`js/particles.min.js`) used to come from Google's and jsDelivr's servers; they are in the repository now, the same files. A script from someone else's server would run with everything typed into the page in reach. Every font file is fetched as the page starts, so that which ones are asked for never depends on what is typed or unlocked.

`index.html` carries a content policy that allows scripts, styles, fonts and images from this site and nothing else: no inline script, nothing from another address, nowhere to send anything. `tools/site.sh` sends the same policy as a response header (`dist/_headers`), along with the ones that keep the page on https and out of other sites' frames. It gathers nothing if `index.html` loads anything from elsewhere, or if the policy there is not, to the letter, the one it sends.

Some things can't be settled from here, only in the Cloudflare dashboard:

- With its Web Analytics and bot detection switched on, Cloudflare adds two scripts of its own to the page as it serves it. The policy stops both from running (the browser's console says so on every visit). Switching them off for this site stops them being added at all.
- Cloudflare Pages keeps every past deployment at an address of its own (`<hash>.<project>.pages.dev`). The page as it was before any of this is still there until those deployments are deleted.

## Building

The page is written in Rust ([Leptos](https://leptos.dev)) and compiled to WebAssembly. `src/` is the source; `pkg/` is the built output that the page loads, and it is committed so the site can be hosted as plain files.

`pkg/fallback/` is the same program translated to plain JavaScript (with Binaryen's `wasm2js`, see `tools/fallback/`). `js/boot.js` loads it instead when the browser can't run WebAssembly: switched off (iOS Lockdown Mode, some locked-down setups) or too old. It is bigger and slower, and nothing else about it differs. Adding `?js` to the page's address runs it in any browser, to try it out.

After changing anything in `src/`:

```sh
./build.sh
```

That builds both. It needs Rust with the `wasm32-unknown-unknown` target, `wasm-pack` (`rustup target add wasm32-unknown-unknown`, `cargo install wasm-pack`), and Node with this project's packages (`npm install`, which brings in Binaryen).

## Deploying

The site is the files `tools/site.sh` gathers into `dist/`: `index.html`, `style_2.css`, `font/`, `js/`, `img/` and `pkg/`, and the `_headers` it writes. Nothing is compiled on the host, so a change under `src/` only goes live once `./build.sh` has been run and `pkg/` committed with it.

In `dist/`, the stylesheet, the fonts, `js/` and `pkg/` sit inside a folder named after their contents (`v-<digest>/`), and the copy of `index.html` there asks for them in that folder. The page and those files only work as a set, and browsers are told to keep `.css` and `.js` files for hours (Cloudflare's Browser Cache TTL) while `index.html` is always fetched fresh. Under fixed names, a returning visitor got the new page with the old stylesheet and the old WebAssembly loader until that cache ran out: new parts of the page unstyled, and the slow JavaScript build running because the old loader didn't fit the new `.wasm`. With the folder's name changing whenever its contents do, that can't happen. `index.html` in the repository keeps the plain paths, so it still runs as it is from a local server.

On Cloudflare Pages: framework preset None, build command `sh tools/site.sh`, build output directory `dist`.

## Testing

`cargo test` runs the crypto, huH codec and compression tests natively, including checks against messages, files and an ML-KEM exchange made by the original JavaScript page (`tests/vectors.json`), against huHs in every compressed form made when each was introduced (`tests/squeezed.json`), and against huHs in today's envelope made when it was introduced (`tests/enveloped.json`). The envelope and the pairing are also held to answers worked out with Node's own PBKDF2, HKDF and AES-256-GCM, so nothing about them belongs to this code alone. The tests for remembered sessions hold one sealed when the format was introduced, which has to keep opening.

On the page, a password or passphrase is stretched by the browser's own `crypto.subtle` (fast, and off the page's thread), which browsers only offer on https and localhost. Anywhere else the same PBKDF2 runs inside the program instead: about a second as WebAssembly, and far longer in the JavaScript build.

A plain file server sends none of the headers the host will, so a page that works from one can still break under its content policy. To try the gathered site as it will be served:

```sh
sh tools/site.sh && python3 tools/serve.py
```

`tests/page-compare.js` walks the page through its states and snapshots every element, to compare two builds of it.
