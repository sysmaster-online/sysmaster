#!/usr/bin/env bash

rustup override set stable
rm -rf vendor
cargo vendor
rustup override unset
grep -i "vendored-sources" .cargo/config
if [ $? -ne 0 ]; then
cat << EOF >> .cargo/config

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

EOF
fi

for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `; do date; sh -x -e $i; done
