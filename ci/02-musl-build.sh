#!/bin/bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals

# install musl-build
rustup target add $arch-unknown-linux-musl

# sudo yum install musl-gcc

# build for musl
# .cargo/config
#[target.$arch-unknown-linux-musl]
#rustflags = ["-C", "target-feature=-crt-static"]

arch=`uname -m`
cargo build --all --release --target=$arch-unknown-linux-musl
cargo test --all --release --target=$arch-unknown-linux-musl
