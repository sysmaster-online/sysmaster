# 此目录为补丁目录,主要应用于对依赖的修改

_注意: 执行vendor时,会将crate-io的源替换成本地vendor目录,如要恢复,请删除.cargo/config中相应的配置_

## 1. 制作补丁
- 推荐`git checkout -b new_patches`创建新分支来操作
- 执行`./vendor.sh`来创建vendor目录和依赖的软件源码
- 在**vendor**下对应的源码目录中提交修改
- 使用`git format-patch -n` 来创建补丁
- 将patch提交开发分支的**patch**目录中, 建议做好序号和命名管理, 必须以patch结尾, 如`0001-fix bug in clap 2.0.patch`

## 2. 发布源码包

- 获取对应版本源码(包含**patch**目录)
- (可选)对相应代码做裁剪定制
- 执行`./vendor.sh`制作对应的**tar.gz**源码发布包
