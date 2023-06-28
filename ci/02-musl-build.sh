#!/usr/bin/env bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals

arch=`uname -m`
# install musl-build
rustup target add $arch-unknown-linux-musl

cargo build --all --no-default-features --features "default" --target=$arch-unknown-linux-musl
