#!/usr/bin/env bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals

arch=`uname -m`
# install musl-build
rustup target add $arch-unknown-linux-musl

# sudo yum install musl-gcc

# build for musl
# .cargo/config
#[target.$arch-unknown-linux-musl]
#rustflags = ["-C", "target-feature=-crt-static"]

cargo build --all --no-default-features --features "default" --target=$arch-unknown-linux-musl
#RUST_BACKTRACE=full RUSTFLAGS="-L /usr/$arch-linux-musl/lib64/libm.a" cargo test --all --all-targets --all-features --target=$arch-unknown-linux-musl -- --nocapture --test-threads=1
