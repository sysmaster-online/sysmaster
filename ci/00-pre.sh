#!/usr/bin/env bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source $SCRIPT_DIR/common_function

contains_chinese

# Install required tools if not already installed
required_packages=("gcc" "openssl-libs" "python3-pip" "python3" "python3-devel" "clang" "libblkid-devel" "kmod-devel" "libselinux-devel")

missing_packages=()
for package in "${required_packages[@]}"; do
    rpm -qi "$package" > /dev/null 2>&1 || missing_packages+=("$package")
done

if [ "${#missing_packages[@]}" -gt 0 ]; then
    sudo sed -i "s:repo.openeuler.org:repo.huaweicloud.com/openeuler:g" /etc/yum.repos.d/*.repo
    sudo yum install --refresh --disablerepo OS --disablerepo EPOL --disablerepo source --disablerepo update --disablerepo EPOL-UPDATE --disablerepo debuginfo -y "${missing_packages[@]}" || exit 1
fi

source ~/.bashrc
cargo -v
if [ $? -ne 0 ]; then
export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
sh rustlang.sh -y --default-toolchain none && sh rustlang.sh -y --default-toolchain none
rm -rf rustlang.sh
fi

source "$HOME/.cargo/env"
rustup default $rust_vendor

# Define the crate names to test
crate_names=("https://github.com/rust-lang/crates.io-index" \
            "https://mirrors.ustc.edu.cn/crates.io-index" \
            "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git" \
            "https://mirrors.sjtug.sjtu.edu.cn/git/crates.io-index")


fastest_source=$(test_fasturl "${crate_names[@]}")

echo "Fastest source: $fastest_source"

# Modify config
mkdir -p ~/.cargo
cat << EOF > ~/.cargo/config
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"

# Use the fastest source
replace-with = 'replace'

[source.replace]
registry = "$fastest_source"

[net]
git-fetch-with-cli = true
EOF

sources=("https://521github.com/" "https://gitclone.com/github.com/" "https://gh.api.99988866.xyz/https://github.com/" "https://github.com/")
url=$(test_fasturl ${sources[@]})
git config --global url."${url}".insteadOf "https://github.com/"


pipurls=("https://pypi.tuna.tsinghua.edu.cn/simple" "http://mirrors.aliyun.com/pypi/simple/" "https://pypi.mirrors.ustc.edu.cn/simple/" "http://pypi.sdutlinux.org/" "http://pypi.douban.com/simple/")
url=$(test_fasturl ${pipurls[@]})

if [[ $url =~ ^https?://([^/]+) ]]; then
    domain="${BASH_REMATCH[1]}"
    pip config set global.index-url $url
    pip config set global.trusted-host $domain
fi

##拉取代码
#rm -rf sysmaster
#git clone https://gitee.com/openeuler/sysmaster.git
#cd sysmaster
#git checkout -b pr_$prid
#git fetch origin pull/$prid/head:master-$prid
#git merge --no-edit master-$prid
