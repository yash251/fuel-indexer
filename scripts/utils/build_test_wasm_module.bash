#!/bin/bash
# 
# Rebuilds all WASM modules used for testing
#
#   Usage: bash scripts/utils/build_test_wasm_module.bash

set -ex

# This is a test index, keep it in assets for now
cargo build -p fuel-indexer-test --release --target wasm32-unknown-unknown
bash scripts/stripper.bash fuel_indexer_test.wasm
cp fuel_indexer_test.wasm packages/fuel-indexer-tests/assets/
rm -fv fuel_indexer_test.wasm

# This is a test index, keep it in assets for now
cargo build -p simple-wasm --release --target wasm32-unknown-unknown
bash scripts/stripper.bash simple_wasm.wasm
cp simple_wasm.wasm packages/fuel-indexer-tests/assets/
rm -fv simple_wasm.wasm

cargo build -p explorer-index --release --target wasm32-unknown-unknown
bash scripts/stripper.bash explorer_index.wasm
cp explorer_index.wasm target/wasm32-unknown-unknown/release/
rm -fv explorer_index.wasm


cargo build -p hello-index --release --target wasm32-unknown-unknown
bash scripts/stripper.bash hello_index.wasm
rm -fv hello_index.wasm

set +ex
