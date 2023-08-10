#!/usr/bin/env bash

# commit-msg hooks
\cp -ar ci/commit-msg .git/hooks

pushd ci
# prepare environment
for i in `ls ci/00-*.sh | sort -u -d `; do sh -x $i; done
 . "$HOME/.cargo/env"

# execute test scripts
for i in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `; do date; sh -x $i; done
popd

# set pre-commit init-templatedir
git config --global init.templateDir ~/.git-template
pre-commit init-templatedir ~/.git-template

# cleanup temporary
cargo clean
git reset --hard

#echo -e "---!!!CHECK cargo-deny !!!---"
#cargo deny -V > /dev/null 2>&1
