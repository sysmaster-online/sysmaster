#!/usr/bin/env bash
set -e
arch=$(uname -m)
vendor="openeuler-22.03LTS_SP1"
force=false

# 处理参数
while [[ $# -gt 0 ]]
do
    key="$1"
    case $key in
        --force)
        force=true
        shift # 移除参数
        ;;
        *)
        if [ $# -ne 1 ]; then
            echo "More than one argument supplied, not supported"
            echo "./build_rpm.sh [openeuler-22.03LTS_SP1]"
            exit 1
        else
            vendor=$1
        fi
        shift # 移除参数
        ;;
    esac
done

# 获取当前脚本的目录
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOTDIR=$SCRIPT_DIR

# 递归向上查找包含 Cargo.toml 文件的目录
while [ ! -f "$ROOTDIR/Cargo.lock" ] && [ "$ROOTDIR" != "/" ]; do
    ROOTDIR=$(dirname "$ROOTDIR")
done

pushd $ROOTDIR
version_line=$(grep -Eo '^version = "[0-9]+\.[0-9]+\.[0-9]+"' ./Cargo.toml)
version=$(echo "$version_line" | awk -F'"' '{print $2}')

TARGETDIR=$ROOTDIR/target/rpms
if [ "$force" = false ]; then
    rm -rf $TARGETDIR
    mkdir -p $TARGETDIR
# 利用cargo vendor创建源码压缩包
    rustup override set stable
    rm -rf vendor
    cargo vendor
    rustup override unset

    # delete large and unused files
    for lib in `find vendor/windows* | grep \\.a$`
    do
        rm -rf $lib
    done
    for lib in `find vendor/winapi* | grep \\.a$`
    do
        rm -rf $lib
    done
    for lib in `find vendor/windows* | grep \\.lib$`
    do
        rm -rf $lib
    done

    rm -rf petgraph/tests

    for crate in `ls -d vendor/win*`
    do
        pushd $crate/src
        if [ $? -ne 0 ] ;then
            continue;
        fi
        for pathToDelete in `ls`
        do
            if [ -d "$pathToDelete" ]; then
                echo "Deleting files in $pathToDelete..."
                rm -rf "$pathToDelete"
            else
                echo "$pathToDelete is not dir."
            fi
        done
        popd
    done

    # compress sysmaster
    pushd $ROOTDIR/../
        rm -rf sysmaster-$version
        cp -a $(basename $ROOTDIR) sysmaster-$version
        pushd sysmaster-$version
        cargo clean
        rm -rf .git next docs tools patch target
        sed -i '/\[patch.crates-io.loopdev\]/{N;N;d}' Cargo.toml
        popd > /dev/null 2>&1
        tar -cJvf $TARGETDIR/sysmaster-$version.tar.xz sysmaster-$version
        rm -rf sysmaster-$version
    popd > /dev/null 2>&1
fi
set +e
# 构建srpm
sudo dnf install -y mock rpm-build createrepo
sudo groupadd mock
sudo usermod -a -G mock $(who | awk '{print $1}' | sort -u)
cp -a $SCRIPT_DIR/* $TARGETDIR
mock -r $vendor-$arch --configdir $TARGETDIR --no-clean --isolation simple --buildsrpm --spec $TARGETDIR/sysmaster.spec  --sources=$TARGETDIR/sysmaster-$version.tar.xz --resultdir $TARGETDIR

# rebuild构建rpms, 结果输出到target/rpms目录下
srpms=$(ls $TARGETDIR/sysmaster-*.src.rpm)
mock -r $vendor-$arch --configdir $TARGETDIR --no-clean --isolation simple --rebuild  $srpms --resultdir $TARGETDIR
createrepo_c $TARGETDIR
popd
