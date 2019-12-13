#!/bin/bash

rm -r near-old-stations || true
rm -r *.gz || true
rm -r .cache.json || true

set -eu

mkdir near-old-stations

cargo r --release -- --mode oneshot
cargo package

cp target/release/near-old-stations.exe near-old-stations/
cp target/release/stats.exe near-old-stations/
cp LICENSE-APACHE near-old-stations/
cp LICENSE-MIT near-old-stations/
cp README.md near-old-stations/
cp CHANGELOG.md near-old-stations/
cp config.sample.toml near-old-stations/config.toml
cp coordinates.json.gz near-old-stations/