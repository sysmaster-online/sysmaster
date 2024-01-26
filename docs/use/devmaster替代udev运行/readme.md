# 使用devmaster替换udev启动

使用`devmaster`替换`sysmaster`虚拟机启动场景的`udev`，使得虚拟机启动后可以正常激活逻辑卷、挂载分区、加载网络等等。

在`sysmaster`源码根目录下执行如下命令进行安装：

```shell
# sh -x tools/run_with_devmaster/install_devmaster.sh [debug|release]
```

## 测试环境

- 测试的虚拟机环境如下：

```shell
# uname -a
Linux localhost.localdomain 5.10.0-60.18.0.50.oe2203.x86_64 #1 SMP Wed Mar 30 03:12:24 UTC 2022 x86_64 x86_64 x86_64 GNU/Linux
```

## 安装方法

1. 参考[`sysmaster`虚拟机启动方案](<http://127.0.0.1:8000/use/%E8%99%9A%E6%9C%BA%E4%B8%AD%E6%9B%BF%E4%BB%A3pid1%E8%BF%90%E8%A1%8C/readme/>)搭建启动环境。

2. 安装`devmaster`的二进制以及相关配置文件，在`sysmaster`源码根目录下执行如下命令：

   ```shell
   # sh -x docs/use/devmaster替代udev运行/install_devmaster.sh [debug|release]
   ```

3. 重启系统，选择`sysmaster`启动项。进入主系统后检查`devmaster`服务状态和网络连通情况。

## 替换initrd中的udev

devmaster提供了dracut模块，用于制作initramfs时替换默认的udev组件。安装devmaster后，执行以下命令制作initramfs：

```shell
# dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd rngd dbus-daemon dbus network-manager plymouth" --add "devmaster"
```
