#!/bin/bash
# try install 3 times

function finish() {
    echo "--- PLEASE RUN sh -x ci/01-pre-commit.sh FIRST IN YOUR LOCALHOST!!! ---"
    # remove tmp
    for rustlist in `git diff --name-only HEAD~$changenum | grep \.rs$ | tr '\n' ' '`
    do
    sed -i '/#!\[deny(missing_docs)]/d' $rustlist 2>/dev/null || true
    sed -i '/#!\[deny(clippy::all)]/d' $rustlist 2>/dev/null || true
    sed -i '/#!\[deny(warnings)]/d' $rustlist 2>/dev/null || true
    done
}

trap finish EXIT
pip3 install pre-commit -i http://mirrors.aliyun.com/pypi/simple/ || pip3 install  -i https://pypi.tuna.tsinghua.edu.cn/simple/ pre-commit || pip3 install pre-commit

## one PR ? Commit
oldnum=`git rev-list origin/master --count`
newnum=`git rev-list HEAD --count`
changenum=$[newnum - oldnum]

# add doc for src code
for rustlist in `git diff --name-only HEAD~$changenum | grep \.rs$ | tr '\n' ' '`
do
egrep '#!\[deny\(missing_docs\)\]' $rustlist || sed -i '1i\#![deny(missing_docs)]' $rustlist 2>/dev/null || true
egrep '#!\[deny\(clippy::all\)\]' $rustlist || sed -i '1i\#![deny(clippy::all)]' $rustlist 2>/dev/null || true
egrep '#!\[deny\(warnings\)\]' $rustlist || sed -i '1i\#![deny(warnings)]' $rustlist 2>/dev/null || true
done

# run base check
filelist=`git diff --name-only HEAD~$changenum HEAD | tr '\n' ' '`
export PATH="$PATH:/home/jenkins/.local/bin"
pre-commit run -vvv --files ${filelist}
