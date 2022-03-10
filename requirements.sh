#!/bin/bash

#1.check pre-commit
#echo -e "---!!!CHECK pre-commit !!!---"
pre-commit --version && [ -e .git/hooks/pre-commit ]
if [ $? -ne 0 ]; then
    cat << EOF
你的环境缺少pre-commit hook, 你可以:
参考：https://pre-commit.com/#rust

1.安装pre-commit...
pip install pre-commit
或
brew install pre-commit
或
conda install -c conda-forge pre-commit

2.安装git pre-commit hook...
pre-commit install
EOF
echo "5s 后自动安装！！！"
sleep 5
pip install pre-commit
pre-commit install
git config --global init.templateDir ~/.git-template
pre-commit init-templatedir ~/.git-template
fi

#echo -e "---!!!CHECK cargo-deny !!!---"
cargo deny -V > /dev/null 2>&1
if [ $? -ne 0 ]; then
    cat << EOF
你的环境缺少cargo deny工具, 你可以:
参考：https://github.com/EmbarkStudios/cargo-deny

1. 安装cargo deny
cargo install --locked cargo-deny
# Or, if you're an Arch user
yay -S cargo-deny
EOF
exit 1
fi
