#!/bin/bash

## one PR ? Commit
# oldnum=`git rev-list origin/master --no-merges --count`
# newnum=`git rev-list HEAD --no-merges --count`
# changenum=$[newnum - oldnum]

# do not use chinese in source code
for rustlist in `git diff origin/master --stat | awk '{print $1}' | grep \.rs$ | tr '\n' ' '`
do
    grep -Pn '[\p{Han}]' $rustlist  && echo "DO NOT USE CHANESE CHARACTERS in code, 不要在源码中使用中文!" && exit 1
done

# install needed tools
sudo yum clean all
sudo yum install --disablerepo everything --disablerepo EPOL --disablerepo source --disablerepo update --disablerepo EPOL-UPDATE --disablerepo debuginfo  -y gcc openssl-libs python3-pip

#git加速并安装rust工具链
git config --global url."https://github.91chi.fun/https://github.com/".insteadOf "https://github.com/"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
sh rustlang.sh -y
rm -rf rustlang.sh

source ~/.bashrc

##Fix cargo clippy timeout : replace cargo crates with ustc
# Modify config
cat << EOF > ~/.cargo/config
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"
# 指定镜像
replace-with = 'ustc'
# 中国科学技术大学
[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"
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
