#!/bin/sh

set -ex
HERE=$(dirname "$0")

cd "$HERE/cachewarmer"
for i in $(seq 0 10)
do
	cargo run --release --bin level"$i" -- urls.txt
done
