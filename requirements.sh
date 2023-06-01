#!/usr/bin/env bash

set -x
# install needed tools
#sudo yum clean all
sudo yum install -y gcc openssl-libs python3-pip musl-gcc

#git加速并安装rust工具链
git config --global url."https://github.91chi.fun/https://github.com/".insteadOf "https://github.com/"
rustup --version
if [ $? -ne 0 ]; then
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
sh rustlang.sh -y
rm -rf rustlang.sh
source ~/.bashrc
fi

# add musl target
arch=`uname -m`
rustup target add $arch-unknown-linux-musl

# Delete cache
rm -rf  ~/.cargo/.package-cache

#check pre-commit
pip3 install pre-commit ruamel.yaml
pre-commit install
git config --global init.templateDir ~/.git-template
pre-commit init-templatedir ~/.git-template

# commit-msg hooks
\cp -ar ci/commit-msg .git/hooks

#echo -e "---!!!CHECK cargo-deny !!!---"
#cargo deny -V > /dev/null 2>&1
