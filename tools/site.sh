#!/bin/sh
# Gathers the files the site is made of into ./dist, so that only those get
# published (not the Rust source, the tests or the build tools).
#
# This is the whole "build" the host runs. On Cloudflare Pages:
#   Build command:           sh tools/site.sh
#   Build output directory:  dist
#
# Nothing is compiled here. pkg/ is made by ./build.sh on your own machine and
# committed, so the host needs no Rust.
set -e
cd "$(dirname "$0")/.."

for needed in index.html style_2.css pkg/huh_encrypter.js pkg/huh_encrypter_bg.wasm pkg/fallback/huh_encrypter.js pkg/fallback/huh_encrypter_bg.js pkg/fallback/huh_encrypter_bg.wasm.js; do
    if [ ! -f "$needed" ]; then
        echo "error: $needed is missing. Run ./build.sh and commit pkg/." >&2
        exit 1
    fi
done

rm -rf dist
mkdir dist
cp -R index.html style_2.css font img pkg dist/

echo "site gathered in dist/:"
find dist -type f | sort
