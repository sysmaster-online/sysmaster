#!/bin/bash
pip3 install pre-commit
filelist=`git diff --name-only HEAD~20 HEAD | tr '\n' ' '`
export PATH="$PATH:/home/jenkins/.local/bin"
pre-commit run -vvv --files ${filelist}
