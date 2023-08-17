#!/bin/bash

# Usage:
# Run the script in the root directory of the sysmaster project, for example:
#   $ sh ./tools/generate_cov.sh

# Install tools
echo "Installing tools..."
rustup override set stable > /dev/null 2>&1
cargo install grcov > /dev/null 2>&1
rustup component add llvm-tools-preview > /dev/null 2>&1

# Ensure build and test succeed
find . -name "*.profraw" | xargs rm -f
cargo clean
rustup override set 1.57.0 > /dev/null 2>&1

echo "Starting to build..."
export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="grcov-sysmaster-%p-%m.profraw"
cargo build --all
if [ $? -ne 0 ]; then
    echo "cargo build failed, exit."
    exit 1
fi

# Ensure test succeed
rm -rf /usr/lib/sysmaster_cov_back > /dev/null 2>&1
mv /usr/lib/sysmaster /usr/lib/sysmaster_cov_back > /dev/null 2>&1
mkdir -p /usr/lib/sysmaster/plugin
cp ./config/conf/plugin.conf /usr/lib/sysmaster/plugin/
cp ./target/debug/*.so /usr/lib/sysmaster/plugin
for i in `find . -name "*.service"`; do cp $i /usr/lib/sysmaster/; done
for i in `find . -name "*.target"`; do cp $i /usr/lib/sysmaster/; done

echo "Starting to test..."
cargo test --all-targets --all -v -- --nocapture --test-threads=1
result=$?
rm -rf /usr/lib/sysmaster
mv /usr/lib/sysmaster_cov_back /usr/lib/sysmaster > /dev/null 2>&1
if [ $result -ne 0 ]; then
    echo "cargo test failed, exit."
    exit 1
fi

# Ensure grcov succeed
rustup override set stable > /dev/null 2>&1

echo "Starting to generate coverage..."
# To be compatible with the sysmaster of src-openeuler
grcov . --ignore "/**/*" --ignore "tests/**/*" --ignore "target/**/*" --ignore "vendor/**/*" --ignore "ci/**/*" --ignore "tools/**/*" --ignore "docs/**/*" --ignore "**/examples/**/*" -s . --binary-path ./target/debug/ -t lcov --branch --ignore-not-existing -o ./cov.info
if [ $? -ne 0 ]; then
    echo "grcov failed, exit."
    exit 1
fi
genhtml -o coverage cov.info
if [ $? -ne 0 ]; then
    echo "genhtml failed, exit."
    exit 1
fi

find . -name "*.profraw" | xargs rm -f
cargo clean
rustup override set 1.57.0 > /dev/null 2>&1
