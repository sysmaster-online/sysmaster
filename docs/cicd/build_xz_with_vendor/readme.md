# 使用Vendor工具发布

在 Rust 项目中，依赖项管理是一个关键的环节。Cargo 是 Rust 的官方包管理工具，它可以自动下载和管理项目的依赖项。然而，有时候您可能需要在没有网络连接的环境中构建项目，或者希望将项目及其所有依赖项打包为一个自包含的分发包。这时，Cargo Vendor 工具就派上用场了。

Cargo Vendor 允许您将项目的所有依赖项和相关文件打包到本地目录中，以便随项目一起发布。这个本地目录通常被称为 "vendor" 目录。以下是如何使用 Cargo Vendor 工具来发布 Rust 项目的一般步骤：

## 步骤 1：安装 Cargo Vendor

如果您尚未安装 Cargo Vendor，可以使用以下命令进行安装：

```shell
cargo install cargo-vendor
```

## 步骤 2：创建 vendor 目录
在项目的根目录中创建一个名为 `vendor` 的目录。

## 步骤 3：运行 Cargo Vendor
在 `vendor` 目录中运行以下命令：

```shell
cargo vendor
```
在此过程中, 可以删除多余的目录, 如`.git`, `docs`等.

## 步骤 4：构建和发布项目
将整个源码仓库打包

!!! tips
    本项目也可以使用同级目录下的脚本[vendor.sh](./vendor.sh)自动化的做这些事情.
