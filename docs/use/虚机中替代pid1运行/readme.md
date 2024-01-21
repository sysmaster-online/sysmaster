# 虚拟机中以pid1运行

## 概述

本教程基于`openEuler LTS 22.03`版本进行实践，理论上适用于其他`Linux`发行版本。首先通过`dracut`命令制作剔除了`systemd`的`initramfs`镜像，然后基于该镜像添加新的`grub`启动条目，并在该启动条目中以`sysmaster-init`作为一号进程实现虚拟机环境的启动初始化。


## 部署和使用

### `initramfs`镜像制作

1. 为了避免`initrd`阶段`systemd`的影响，需要制作一个剔除`systemd`的`initramfs`镜像，并以该镜像进入`initrd`流程。使用如下命令：

```
# dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd" /boot/initrd_withoutsd.img
```

2. 得到上述`initramfs`后，在`grub.cfg`中增加新的启动项，`aarch64`下的路径为`/boot/efi/EFI/openEuler/grub.cfg`，`x86_64`下的路径为`/boot/grub2/grub.cfg`：

```
...
### BEGIN /etc/grub.d/10_linux ###
menuentry 'openEuler (5.10.0-60.18.0.50.oe2203.aarch64) 22.03 LTS' --class openeuler --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'gnulinux-5.10.0-60.18.0.50.oe2203.aarch64-advanced-53b0b401-a14c-43f1-9d96-9a66143bbb17' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt2'
        if [ x$feature_platform_search_hint = xy ]; then
          search --no-floppy --fs-uuid --set=root --hint-bios=hd0,gpt2 --hint-efi=hd0,gpt2 --hint-baremetal=ahci0,gpt2  2a438da5-a305-4682-a45f-5cee6f02c3c6
        else
          search --no-floppy --fs-uuid --set=root 2a438da5-a305-4682-a45f-5cee6f02c3c6
        fi
        echo    'Loading Linux 5.10.0-60.18.0.50.oe2203.aarch64 ...'
        linux   /vmlinuz-5.10.0-60.18.0.50.oe2203.aarch64 root=/dev/mapper/openeuler-root ro rd.lvm.lv=openeuler/root rd.lvm.lv=openeuler/swap video=VGA-1:640x480-32@60me console=tty0 crashkernel=1024M,high smmu.bypassdev=0x1000:0x17 smmu.bypassdev=0x1000:0x15 video=efifb:off
        echo    'Loading initial ramdisk ...'
        initrd  /initramfs-5.10.0-60.18.0.50.oe2203.aarch64.img
}
### 新增如下启动项 ###
menuentry 'Boot with sysmaster' --class openeuler --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'gnulinux-5.10.0-60.18.0.50.oe2203.aarch64-advanced-53b0b401-a14c-43f1-9d96-9a66143bbb17' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt2'
        if [ x$feature_platform_search_hint = xy ]; then
          search --no-floppy --fs-uuid --set=root --hint-bios=hd0,gpt2 --hint-efi=hd0,gpt2 --hint-baremetal=ahci0,gpt2  2a438da5-a305-4682-a45f-5cee6f02c3c6
        else
          search --no-floppy --fs-uuid --set=root 2a438da5-a305-4682-a45f-5cee6f02c3c6
        fi
        echo    'Loading Linux 5.10.0-60.18.0.50.oe2203.aarch64 ...'
        linux   /vmlinuz-5.10.0-60.18.0.50.oe2203.aarch64 root=/dev/mapper/openeuler-root rw rd.lvm.lv=openeuler/root rd.lvm.lv=openeuler/swap video=VGA-1:640x480-32@60me console=tty0 crashkernel=1024M,high smmu.bypassdev=0x1000:0x17 smmu.bypassdev=0x1000:0x15 video=efifb:off init=/init plymouth.enable=0
        echo    'Loading initial ramdisk ...'
        initrd  /initrd_withoutsd.img
}
...
```

3. 增加启动项时，拷贝一份原有启动项，并做以下几处修改：

- 新增启动项需要避免和原启动项重名，例如上述案例中设置为`Boot with sysmaster`：

```
menuentry 'Boot with sysmaster'
```

- 内核启动参数需要修改`root=/dev/mapper/openeuler-root ro`为`root=/dev/mapper/openeuler-root rw`。因为目前`sysmaster`并未实现在切根后重新挂载根分区的功能，所以小系统中的根分区需要挂载成`rw`。

- 启动参数中需要显示指定一号进程`init=/init`，在安装脚本`install_sysmaster.sh`中，我们会生成`/init`软链接指向`sysmaster-init`，从而避免内核以`systemd`作为1号进程启动。

- 如果环境上安装了plymouth，需要添加`plymouth.enable=0`禁用plymouth，否则plymouth进程会残留到大系统影响用户在虚拟终端正常登录。

- `initrd`项需要对应修改为`initrd  /initrd_withoutsd.img`，此`img`为步骤1生成。


### 安装`sysmaster`

1. 构建`sysmaster`的`debug`或`release`二进制版本：

```
# cargo build --all [--release]
```

1. 在源码根目录下使用安装脚本`install_sysmaster.sh`将`sysmaster`的二进制文件、系统服务、配置文件等安装到系统中，执行以下命令：

```
# sh -x docs/use/虚机中替代pid1运行/install_sysmaster.sh [debug|release]
```

可以指定安装`debug`或`release`版本，未指定时默认安装`debug`二进制版本。

3. 启动串口登陆服务：

根据体系结构启动对应的串口登陆服务，`aarch64`对应`serial-getty-ttyAMA0.service`，`x86_64`对应`serial-getty-ttyS0.service`。此服务主要针对有`console`串口的情况，例如`virsh console`进入串口，私人笔记本创建的虚拟机，默认应该都是只有`tty1`。

开机启动后手动执行`sctl start`命令启动串口登陆服务，或者将该服务添加到`multi-user.target`配置的`Requires`字段中实现开机自启。

```
# sctl start <serial-getty-ttyAMA0.service|serial-getty-ttyS0.service>
```

### 系统启动

虚拟机重启，在`grub`引导启动界面选择对应的启动项。启动后，可以通过`tty1`或者`ssh`登陆。


## 卸载`sysmaster`

执行以下命令，可以从系统中卸载`sysmaster`：

```
# rm -rf /lib/sysmaster
# rm -rf /etc/sysmaster
# unlink /init
```
