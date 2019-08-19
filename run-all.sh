#!/bin/sh

set -ex
HERE=$(dirname "$0")

cd "$HERE/cachewarmer"
for i in $(seq 0 12)
do
	cargo run --release --bin level"$i" -- urls.txt
done

cd ../cachewarmer-nightly
cargo +nightly run --release --bin level13 -- ../cachewarmer/urls.txt
