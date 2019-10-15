#!/bin/bash

rm -r release || true

set -eu

cargo b --release
mkdir release
cp target/release/near-old-stations.exe release/
cp config.toml.sample release/config.toml