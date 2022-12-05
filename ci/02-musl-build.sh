#!/bin/bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals

arch=`uname -m`
# install musl-build
rustup target add $arch-unknown-linux-musl

# sudo yum install musl-gcc

# build for musl
# .cargo/config
#[target.$arch-unknown-linux-musl]
#rustflags = ["-C", "target-feature=-crt-static"]

cargo build --all --release --target=$arch-unknown-linux-musl
#RUST_BACKTRACE=full cargo test --all --target=$arch-unknown-linux-musl -- --nocapture --test-threads=1
