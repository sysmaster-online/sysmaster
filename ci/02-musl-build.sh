#!/usr/bin/env bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals
echo "(Optional) Build sysMaster with musl."
export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup

arch=`uname -m`
# install musl-build
rustup target add $arch-unknown-linux-musl

cargo build --all --no-default-features --features "default" --target=$arch-unknown-linux-musl
