#!/bin/bash
sudo yum clean all
sudo yum install -y gcc openssl-libs

#git加速并安装rust工具链
git config --global url."https://github.91chi.fun/https://github.com/".insteadOf "https://github.com/"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
sh rustlang.sh -y

source ~/.bashrc


##拉取代码
#rm -rf process1
#git clone https://gitee.com/openeuler/process1.git
#cd process1
#git checkout -b pr_$prid
#git fetch origin pull/$prid/head:master-$prid
#git merge --no-edit master-$prid
