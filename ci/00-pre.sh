#!/usr/bin/env bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source $SCRIPT_DIR/common_function

# 定义标志文件的路径
flag_file="$SCRIPT_DIR/../.git/sysmaster-first-build.flag"

# 检查标志文件是否存在
if [ -e "$flag_file" ]; then
    echo "Not first build, skipping."
    exit 0
else
    echo "This is the first build, continue."
    touch "$flag_file"
fi

# Function to check if a file contains Chinese characters
contains_chinese() {
    grep -Pn '[\p{Han}]' "$1" && echo "DO NOT USE CHINESE CHARACTERS in code, 不要在源码中使用中文!" && exit 1
}

# Check for Chinese characters in modified Rust files
git diff origin/master --name-only | grep -F '.rs' | while IFS= read -r rustfile; do
    contains_chinese "$rustfile"
done

# Install required tools if not already installed
if [ ! -f "/etc/centos-release" ] && [ ! -f "/etc/fedora-release" ]; then
    required_packages=("gcc" "openssl-libs" "python3-pip" "python3" "python3-devel" "clang" "util-linux-devel" "kmod-devel")
else
    required_packages=("gcc" "openssl-libs" "python3-pip" "python3" "python3-devel" "clang" "libblkid-devel" "kmod-devel" "libselinux-devel")
fi

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
sh rustlang.sh -y --default-toolchain none
rm -rf rustlang.sh
fi

source "$HOME/.cargo/env"
rustup default 1.57

# Define the crate names to test
crate_names=("https://github.com/rust-lang/crates.io-index" \
            "https://mirrors.ustc.edu.cn/crates.io-index" \
            "https://rsproxy.cn/crates.io-index" \
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

rm -rf  ~/.cargo/.package-cache

##拉取代码
#rm -rf sysmaster
#git clone https://gitee.com/openeuler/sysmaster.git
#cd sysmaster
#git checkout -b pr_$prid
#git fetch origin pull/$prid/head:master-$prid
#git merge --no-edit master-$prid
