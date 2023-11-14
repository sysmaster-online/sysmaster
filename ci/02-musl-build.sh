#!/usr/bin/env -e bash

#RUST_BACKTRACE=1 cargo run --release -p sysmaster --example signals
echo "(Optional) Build sysMaster with musl."
export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup

# Install required tools if not already installed
required_packages=("musl-gcc")
missing_packages=()
for package in "${required_packages[@]}"; do
    rpm -qi "$package" > /dev/null 2>&1 || missing_packages+=("$package")
done

if [ "${#missing_packages[@]}" -gt 0 ]; then
    sudo sed -i "s:repo.openeuler.org:repo.huaweicloud.com/openeuler:g" /etc/yum.repos.d/*.repo
    sudo yum install --refresh --disablerepo OS --disablerepo EPOL --disablerepo source --disablerepo update --disablerepo EPOL-UPDATE --disablerepo debuginfo -y "${missing_packages[@]}" || exit 1
fi

arch=`uname -m`
# install musl-build
rustup target add $arch-unknown-linux-musl

cargo build --all --no-default-features --features "default" --target=$arch-unknown-linux-musl
