#!/bin/bash
# try install 3 times
pip3 install pre-commit -i http://mirrors.aliyun.com/pypi/simple/ || pip3 install  -i https://pypi.tuna.tsinghua.edu.cn/simple/ pre-commit || pip3 install pre-commit
#rustlist=`git diff --name-only HEAD~5 HEAD | grep \.rs$ | tr '\n' ' '`
#grep -P '[\p{Han}]' $rustlist && echo "rust 源码文件中禁用中文字符" && exit
filelist=`git diff --name-only HEAD~20 HEAD | tr '\n' ' '`
export PATH="$PATH:/home/jenkins/.local/bin"
pre-commit run -vvv --files ${filelist}
