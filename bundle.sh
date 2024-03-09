#!/bin/bash


out_path="out/kavimo-download.exe"

rm -rf out
mkdir -p out 
cargo b -r
cp target/release/kavimo-download.exe "$out_path"
upx --best --lzma "$out_path"
sha256sum "$out_path" > "$out_path.sha256"
sha1sum "$out_path" > "$out_path.sha1"
gpg --armor --detach-sign "$out_path"
