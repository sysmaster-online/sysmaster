#!/usr/bin/env bash

# cargo vendor
echo "cargo vendor ..."
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
for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `
do
    date; sh -x $i;
done

# cleanup temporary
cargo clean
git checkout -- .cargo/config

# echo sysMaster version
echo "Create a compressed archive of tar.gz ..."
version_line=$(grep -Eo '^version = "[0-9]+\.[0-9]+\.[0-9]+"' ./Cargo.toml)
version=$(echo "$version_line" | awk -F'"' '{print $2}')
echo "You can create sysmaster-$version.tar.gz by using the tar -cJvf command."

echo "You can replace crates.io with vendored-sources in .cargo/config!!!"
cat .cargo/config
