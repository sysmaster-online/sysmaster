#!/usr/bin/env bash

# 获取当前脚本的目录
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# 递归向上查找包含 Cargo.toml 文件的目录
while [ ! -f "$SCRIPT_DIR/Cargo.lock" ] && [ "$SCRIPT_DIR" != "/" ]; do
    SCRIPT_DIR=$(dirname "$SCRIPT_DIR")
done

# cargo vendor
echo "cargo vendor ..."
pushd $SCRIPT_DIR
rustup override set stable
rm -rf vendor
cargo vendor

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

rustup override unset

echo "set replace crates.io in .cargo/config ..."
grep -i "vendored-sources" .cargo/config
if [ $? -ne 0 ]; then
cat << EOF >> .cargo/config

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF
fi

echo "Applying patches in patch directory ..."
for i in `ls patch/*.patch | sort -u -d`
do
    git am $i;
done

echo "Cargo build ..."
set -e
for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `
do
    date; sh -x $i;
done
set +e

# cleanup temporary
cargo clean
git checkout -- .cargo/config

echo "You can replace crates.io with vendored-sources in .cargo/config!!!"
cat .cargo/config

# echo sysMaster version
echo "Create a compressed archive of tar.gz ..."
version_line=$(grep -Eo '^version = "[0-9]+\.[0-9]+\.[0-9]+"' ./Cargo.toml)
version=$(echo "$version_line" | awk -F'"' '{print $2}')
echo "You can create sysmaster-$version.tar.gz by using the tar -cJvf command."
popd

# compress sysmaster
pushd $SCRIPT_DIR/../
    rm -rf sysmaster-$version
    cp -a sysmaster sysmaster-$version
    pushd sysmaster-$version
    cargo clean
    rm -rf .git next docs tools patch target
    sed -i '/\[patch.crates-io.loopdev\]/{N;N;d}' Cargo.toml
    popd > /dev/null 2>&1
    tar -cJvf sysmaster-$version.tar.xz sysmaster-$version
    echo "You can find sysmaster-$version.tar.xz in the ${PWD}."
    rm -rf sysmaster-$version
popd > /dev/null 2>&1
