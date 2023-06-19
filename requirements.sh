#!/usr/bin/env bash

# commit-msg hooks
\cp -ar ci/commit-msg .git/hooks

pushd ci
#准备环境
for i in `ls ci/00-*.sh | sort -u -d `; do sh -x $i; done
 . "$HOME/.cargo/env"

#执行测试
for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `; do date; sh -x -e $i; done
popd

git config --global init.templateDir ~/.git-template
pre-commit init-templatedir ~/.git-template

#echo -e "---!!!CHECK cargo-deny !!!---"
#cargo deny -V > /dev/null 2>&1
