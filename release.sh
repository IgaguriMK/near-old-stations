#!/bin/bash

rm -r near-old-stations || true

set -eu

cargo b --release
mkdir near-old-stations
cp target/release/near-old-stations.exe near-old-stations/
cp config.sample.toml near-old-stations/config.toml