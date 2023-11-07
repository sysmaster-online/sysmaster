# 使用Mock工具构建RPM包

Mock工具是一个用于构建RPM包的强大工具，它可以帮助你在隔离的环境中构建RPM包，以确保包的依赖关系和构建过程的一致性。这里将介绍如何使用Mock工具从源码构建RPM包。

## 步骤 1: 安装Mock

首先，确保你的系统上已经安装了Mock工具。你可以使用包管理工具（如`yum`或`dnf`）来安装Mock，例如：

```shell
sudo dnf install mock rpm-build
```

## 步骤 2: 配置Mock
Mock使用配置文件来定义构建环境，你需要创建一个配置文件以指定构建的目标（目标发行版、架构等）以及其他配置选项。配置文件通常存放在/etc/mock/目录中。你可以复制一个现有的配置文件并根据需要进行调整。
```
sudo cp /etc/mock/default.cfg /etc/mock/myconfig.cfg
```
然后，编辑myconfig.cfg文件以设置正确的目标和其他选项。sysMaster已经提供一份openeuler的配置.

## 步骤 3: 准备源代码, 可以使用vendor来打包压缩一份源码包.
将你的源代码和spec文件（RPM包的描述文件）准备好，并确保它们位于合适的目录中。通常，你需要创建一个目录结构，包括SOURCES和SPECS目录。

## 步骤 4: 使用Mock构建
使用以下命令来使用Mock工具构建RPM包：

```shell
TARGETDIR="target/rpms"
mock -r $vendor-$arch --configdir $TARGETDIR --no-clean --isolation simple --buildsrpm --spec $TARGETDIR/sysmaster.spec  --sources=$TARGETDIR/sysmaster-$version.tar.xz --resultdir $TARGETDIR


-r $vendor-$arch：指定Mock配置文件的名称。
--rebuild：告诉Mock工具要重新构建RPM包。
$TARGETDIR/sysmaster-$version.tar.xz：你的RPM包的sources文件的路径。
```

Mock将在指定的构建环境中自动解决依赖关系、下载构建所需的源代码，并执行构建操作。构建过程完成后，你将在Mock工具的输出目录中找到生成的RPM包。

## 步骤 5: 获取生成的RPM包
Mock工具的输出目录通常为/var/lib/mock/<config-name>/result/，你可以在该目录中找到构建成功的RPM包。

!!! note
    sysmaster提供[自动化脚本](./build_rpm.sh)来帮助生成rpms,sysmaster提供的脚本输出到target/rpms
