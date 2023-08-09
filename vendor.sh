#!/usr/bin/env bash

echo "cargo vendor ..."
rustup override set stable
rm -rf vendor
cargo vendor
rustup override unset

echo "set vendor replace crates.io ..."
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
    git am $i
done

echo "Cargo build ..."
for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `
do
    date; sh -x -e $i;
done

echo "Create a compressed archive of tar.gz ..."
version_line=$(grep -Eo 'version = "[0-9]+\.[0-9]+\.[0-9]+"' ./Cargo.toml)
version=$(echo "$version_line" | awk -F'"' '{print $2}')
echo "You can create sysmaster-$version.tar.gz by using the tar -zcvf command."
