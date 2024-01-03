#!/usr/bin/env bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source $SCRIPT_DIR/common_function

#获取allow crates
ALLOWED_CRATES=$(cat $SCRIPT_DIR/crates.allow | grep -e "^-" | cut -d ':' -f 1 | cut -d '-' -f 2)

# 提取Cargo.lock中新增的依赖项
DEPS=$(git diff origin/master --name-only | grep \.lock$ | xargs -i git diff {} | grep -e "^+name =" | cut -d '=' -f 2)

# 检查每个依赖项
for DEP in $DEPS; do
    if ! grep -q "^$DEP$" <<< "$ALLOWED_CRATES"; then
        echo "非法crate被发现: $DEP"
        exit 1
    fi
done

echo "所有依赖项都是允许的。"
