#!/usr/bin/env bash

case $1 in
  -d|--debug)
    cargo build --manifest-path ./Cargo.toml -p bdk-coin-select-simulation
    rust-gdb --args ./target/debug/bdk-coin-select-simulation ./random_blocks.csv
    exit 1
    ;;
  -b|--build)
    cargo build --manifest-path ./Cargo.toml -p bdk-coin-select-simulation
    exit 1
    ;;
  *)
    cargo build --manifest-path ./Cargo.toml -p bdk-coin-select-simulation --release
    cargo run --manifest-path ./Cargo.toml -r -p bdk-coin-select-simulation -- ./data/scenarios/bustabit-2019-2020-tiny.csv ./simulation_results
    exit 1
    ;;
esac
