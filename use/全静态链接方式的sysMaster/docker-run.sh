#!/usr/bin/env bash

#1.check cargo musl build
#echo -e "---!!!CHECK CARGO AND DOCKER!!!---"
rustup show | grep -i x86_64-unknown-linux-musl > /dev/null 2>&1
if [ $? -ne 0 ]; then
    cat << EOF
你的环境缺少rust构建工具, 你可以:

1.安装rust环境...
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

2.安装x86_64-unknown-linux-musl...
rustup target add x86_64-unknown-linux-musl

3.安装musl-gcc...
dnf install musl-gcc -y
EOF
exit 1
fi

#!.check docker
docker -v > /dev/null 2>&1
if [ $? -ne 0 ]; then
    cat << EOF
你的环境缺少docker工具, 你可以:

1.安装docker环境...
dnf install docker -y
或
curl -sSL https://get.docker.com | sh

2.开启docker服务
systemctl start docker; systemctl enable docker

3.如果不支持cgroupv2, 修改后重启系统
sudo dnf install -y grubby ; sudo grubby --update-kernel=ALL --args="systemd.unified_cgroup_hierarchy=0"
EOF
exit 1
fi

#!.build
echo -e "---!!!CARGO BUILD!!!---"
cargo build --target x86_64-unknown-linux-musl || exit 1

#!.docker build
echo -e "\n\n\n---!!!DOCKER BUILD!!!---"
docker stop prun
docker rmi sysmaster -f > /dev/null 2>&1
ln -s ../../target target
docker build --no-cache --tag sysmaster `pwd` || exit 1

#!.docker run
echo -e "\n\n\n---!!!RUN sysmaster IN DOCKER!!!---"
docker run --rm --name prun --privileged -ti sysmaster init $* || exit 1
