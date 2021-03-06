#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release

near deploy contractspace.testnet --wasmFile target/wasm32-unknown-unknown/release/test_contract.wasm

