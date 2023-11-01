#!/usr/bin/env bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
flag_file="$SCRIPT_DIR/.git/hooks/commit-msg"
set -e

# check flag_file
if [ -e "$flag_file" ]; then
    echo "Not first build, skipping preinstall."
else
    echo "This is the first build, preinstall."
    # set pre-commit init-templatedir
    git config --global init.templateDir ~/.git-template
    pre-commit init-templatedir ~/.git-template
    pre-commit install

    # prepare environment
    for script in `ls ci/00-*.sh | sort -u -d `; do
        sh -x $script;
    done
    . "$HOME/.cargo/env"
    touch "$flag_file"
fi

# execute test scripts
for script in `ls ci/*.sh | sort -u -d | grep -v "00-pre.sh" `; do
    date; sh -x $script;
done
