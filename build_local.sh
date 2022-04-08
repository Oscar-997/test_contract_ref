#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release

near deploy beer333.testnet --wasmFile target/wasm32-unknown-unknown/release/test_contract.wasm

