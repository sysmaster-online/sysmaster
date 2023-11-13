#!/usr/bin/env bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source $SCRIPT_DIR/common_function

for line in `cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | "\(.name):\(.version)"'`
do
    cargo build --package $line
    if [ $? -ne 0 ]; then
        echo "Failed to build $line"
        exit 1
    fi
done
