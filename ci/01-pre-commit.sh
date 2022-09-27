#!/bin/bash
# try install 3 times
pip3 install pre-commit -i http://mirrors.aliyun.com/pypi/simple/ || pip3 install  -i https://pypi.tuna.tsinghua.edu.cn/simple/ pre-commit || pip3 install pre-commit

## one PR ? Commit
oldnum=`git rev-list origin/master --count`
newnum=`git rev-list HEAD --count`
changenum=$[newnum - oldnum]

# add doc for src code
rustlist=`git diff --name-only HEAD~$changenum HEAD | grep \.rs$ | tr '\n' ' '`
sed -i '1i\#![deny(missing_docs)]' $rustlist 2>/dev/null || true

# run base check
filelist=`git diff --name-only HEAD~$changenum HEAD | tr '\n' ' '`
export PATH="$PATH:/home/jenkins/.local/bin"
pre-commit run -vvv --files ${filelist}
