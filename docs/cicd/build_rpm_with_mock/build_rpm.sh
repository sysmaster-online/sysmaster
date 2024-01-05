#!/usr/bin/env bash
set -e
arch=$(uname -m)
vendor="openeuler-22.03LTS_SP1"
force=false
mode="release"
while IFS='=' read -r key value
do
    if [[ $key == "ID" ]]; then
        ID=$value
    elif [[ $key == "VERSION_ID" ]]; then
        VERSION_ID=$value
    fi
done < "/etc/os-release"

# Concatenate the values
vendor="${ID}-${VERSION_ID}"

# 获取当前脚本的目录
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOTDIR=$SCRIPT_DIR
# 递归向上查找包含 Cargo.toml 文件的目录
while [ ! -f "$ROOTDIR/Cargo.lock" ] && [ "$ROOTDIR" != "/" ]; do
    ROOTDIR=$(dirname "$ROOTDIR")
done

# 处理参数
TEMP=`getopt -o fv:m:h --long force,vendor:,mode:,help -n 'build_rpm.sh' -- "$@"`
if [ $? != 0 ] ; then echo "Terminating..." >&2 ; exit 1 ; fi
eval set -- "$TEMP"
while true ; do
    case "$1" in
        -f|--force)
            force=true
            shift
            ;;
        -v|--vendor)
            vendor="$2"
            shift 2
            ;;
        -m|--mode)
            mode="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  -f, --force      强制执行, 跳过源码打包"
            echo "  -m, --mode       设置模式,默认为release,可选值为debug"
            echo "  --vendor         设置供应商,默认为/etc/os-release中$ID-$VERSION_ID,vendor支持的配置在/etc/mock目录下"
            echo "  --help           显示帮助"
            exit 0
            ;;
        --)
            shift
            break
            ;;
        *)
            echo "Unsupported option $1!"
            exit 1
            ;;
    esac
done

real_vendor=
if [[ "$vendor" != *"openeuler"* ]]; then
    configdir=""
fi


pushd $ROOTDIR
version_line=$(grep -Eo '^version = "[0-9]+\.[0-9]+\.[0-9]+"' ./Cargo.toml)
version=$(echo "$version_line" | awk -F'"' '{print $2}')

TARGETDIR=$ROOTDIR/target/rpms
# 利用cargo vendor构建源码包
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

# 构建srpm
sudo dnf install -y mock rpm-build createrepo
sudo groupadd mock | true
sudo usermod -a -G mock $(who | awk '{print $1}' | sort -u) | true
cp -a $SCRIPT_DIR/* $TARGETDIR

if [ "$mode" = "debug" ]; then
    pushd $TARGETDIR
    echo "Mode is set to debug"
    sed -i 's/target\/release/target\/debug/g' sysmaster.spec
    sed -i 's/--profile release/--profile dev/g' sysmaster.spec
    popd
fi

configdir="--configdir $TARGETDIR"
if [[ "$vendor" != *"openeuler"* ]]; then
    configdir=""
fi
mock -r $vendor-$arch $configdir --no-clean --isolation simple --buildsrpm --spec $TARGETDIR/sysmaster.spec  --sources=$TARGETDIR/sysmaster-$version.tar.xz --resultdir $TARGETDIR

# rebuild构建rpms, 结果输出到target/rpms目录下
srpms=$(ls $TARGETDIR/sysmaster-*.src.rpm)
mock -r $vendor-$arch $configdir --no-clean --isolation simple --rebuild  $srpms --resultdir $TARGETDIR
createrepo_c $TARGETDIR
popd

file_path="/etc/yum.repos.d/sysmaster.repo"

# Create the file
sudo touch $file_path

# Write the content to the file
sudo echo "[sysmaster]" >> $file_path
sudo echo "name=My sysMaster Repository" >> $file_path
sudo echo "baseurl=$TARGETDIR" >> $file_path
sudo echo "enabled=1" >> $file_path
sudo echo "gpgcheck=0" >> $file_path

echo "---sysmaster repo created at $file_path, you can use `yum install sysmaster` to install it---"
