#!/bin/sh
# Builds the page into ./pkg, which index.html loads: the WebAssembly build, and
# the same program as plain JavaScript (pkg/fallback) for browsers that can't
# run WebAssembly.
#
# Needs: Rust with the wasm32-unknown-unknown target, wasm-pack, and (for the
# fallback) node with this project's npm packages.
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-pack
#   npm install
set -e
cd "$(dirname "$0")"
# rustup installs here; not every shell has it on PATH.
PATH="$HOME/.cargo/bin:$PATH"

# Rust keeps the source path of every possible panic in what it builds. Shorten
# them, so the published files don't carry this machine's folder names (and come
# out the same wherever they are built). The last matching rule wins.
RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$(pwd)=. ${RUSTFLAGS:-}"
export RUSTFLAGS

wasm-pack build --target web --release --out-dir pkg --no-typescript

# Keep only what the page loads; pkg/ is committed so the site needs no build step to host.
rm -f pkg/.gitignore pkg/package.json pkg/README.md

# Both builds have to come from the same compile, or the two would drift apart.
rm -rf pkg/fallback
tools/fallback/build-fallback.sh target/wasm32-unknown-unknown/release/huh_encrypter.wasm pkg/fallback
node tools/fallback/smoke-test.mjs pkg/fallback

if grep -a -q "$HOME" pkg/huh_encrypter_bg.wasm; then
    echo "error: the build still contains $HOME" >&2
    exit 1
fi

ls -l pkg pkg/fallback
