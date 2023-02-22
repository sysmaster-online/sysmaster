#!/bin/bash

## one PR ? Commit
# oldnum=`git rev-list origin/master --no-merges --count`
# newnum=`git rev-list HEAD --no-merges --count`
# changenum=$[newnum - oldnum]

# do not use chinese in source code
for rustlist in `git diff origin/master --name-only | grep \.rs$ | tr '\n' ' '`
do
    grep -Pn '[\p{Han}]' $rustlist  && echo "DO NOT USE CHANESE CHARACTERS in code, 不要在源码中使用中文!" && exit 1
done

# install needed tools
rpm -qi gcc openssl-libs python3-pip musl-gcc > /dev/null 2>&1
if [ $? -ne 0 ]; then
sudo sed -i "s:repo.openeuler.org:repo.huaweicloud.com/openeuler:g" /etc/yum.repos.d/*.repo
sudo yum install --refresh --disablerepo OS --disablerepo EPOL --disablerepo source --disablerepo update --disablerepo EPOL-UPDATE --disablerepo debuginfo  -y gcc openssl-libs python3-pip musl-gcc
if [ $? -ne 0 ]; then
    exit 1
fi
fi

#git加速并安装rust工具链
# git config --global url."https://gh.api.99988866.xyz/https://github.com/".insteadOf "https://github.com/"
git config --global url."https://gitclone.com/github.com/".insteadOf "https://github.com/"
git clone https://github.com/rust-unofficial/awesome-rust.git
if [ $? -ne 0 ]; then
    git config --unset --global url."https://gitclone.com/github.com/".insteadOf "https://github.com/"
    git config --global url."https://gh.api.99988866.xyz/https://github.com/".insteadOf "https://github.com/"
    git clone https://github.com/rust-unofficial/awesome-rust.git
    if [ $? -ne 0 ]; then
      git config --unset --global url."https://gh.api.99988866.xyz/https://github.com/".insteadOf "https://github.com/"
    fi
fi
rm -rf ./awesome-rust.git



source ~/.bashrc
cargo -v
if [ $? -ne 0 ]; then
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
sh rustlang.sh -y
rm -rf rustlang.sh
fi

source ~/.bashrc

##Fix cargo clippy timeout : replace cargo crates with ustc
arch=`uname -m`
# Modify config
cat << EOF > ~/.cargo/config
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"

# 指定镜像
replace-with = 'ustc'

# 中国科学技术大学
[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"

# 字节跳动
[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[target.$arch-unknown-linux-musl]
rustflags = ["-C", "target-feature=-crt-static"]
EOF

# Delete cache
rm -rf  ~/.cargo/.package-cache

##拉取代码
#rm -rf sysmaster
#git clone https://gitee.com/openeuler/sysmaster.git
#cd sysmaster
#git checkout -b pr_$prid
#git fetch origin pull/$prid/head:master-$prid
#git merge --no-edit master-$prid
