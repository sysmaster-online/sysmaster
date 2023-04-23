# 虚拟机场景运行sysmaster



## 思路

以openEuler LTS 22.03版本为基础镜像创建虚拟机，通过dracut重做initrd，去除systemd影响；同时虚拟机中以sysmaster为init进程，实现以sysmaster为1号进程的虚拟机。

首先需要准备一台openEuler 22.03 LTS虚拟机(理论上来说对于虚拟机版本并没有严格要求，只是我使用此版本进行验证)，再进行后续操作。



## 系统制作

### 小系统制作

**1.**为了避免小系统中systemd的影响，需要重新制作一个不包含systemd的initrd。可以使用如下命令制作：

```
dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd" /boot/initrd_withoutsd.img
```

**2.**得到上述initrd后，修改grub.cfg增加启动项(`aarch64:/boot/efi/EFI/openEuler/grub.cfg;x86_64:/boot/grub2/grub.cfg`)，也避免对原系统有影响：

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
menuentry 'openEuler (5.10.0-60.18.0.50.oe2203.aarch64) 22.03 LTS without systemd' --class openeuler --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'gnulinux-5.10.0-60.18.0.50.oe2203.aarch64-advanced-53b0b401-a14c-43f1-9d96-9a66143bbb17' {
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
        linux   /vmlinuz-5.10.0-60.18.0.50.oe2203.aarch64 root=/dev/mapper/openeuler-root rw rd.lvm.lv=openeuler/root rd.lvm.lv=openeuler/swap video=VGA-1:640x480-32@60me console=tty0 crashkernel=1024M,high smmu.bypassdev=0x1000:0x17 smmu.bypassdev=0x1000:0x15 video=efifb:off init=/init
        echo    'Loading initial ramdisk ...'
        initrd  /initramfs-5.10.0-60.18.0.50.oe2203.aarch64.img
}
...
```

修改点有以下几点注意：

- 名称需要修改，避免与原有启动项混淆，例如我增加了`without systemd`：

```
menuentry 'openEuler (5.10.0-60.18.0.50.oe2203.aarch64) 22.03 LTS without systemd'
```

- linux启动项需要修改`root=/dev/mapper/openeuler-root ro`为`root=/dev/mapper/openeuler-root rw`。因为目前sysmaster并未实现在切根后重新挂载根分区的功能，所以小系统中的根分区需要挂载成`rw`。且需要指定`init=/init`，避免拉起systemd作为1号进程。
- initrd项需要对应修改为`initrd  /initrd_withoutsd.img`，此img为步骤1生成。

至此，小系统修改结束。



## 大系统准备

**1.** 将sysmaster编译二进制拷贝到对应目录，可以通过install_sysmaster.sh进行安装。使用方法是在sysmaster编译目录下，执行`sh install_sysmaster.sh debug`。注意这里将init二进制拷贝到为`/init`。与上面修改的linux启动项相对应。

**2.** 将`run_with_vm`目录下service和target文件拷贝到`/usr/lib/sysmaster`目录下。

**3.** `yum -y install net-tools`，安装ifconfig以及route工具，为步骤4的`setip.service`启动提供准备。

**4.** 修改`/usr/lib/sysmaster/basic.target`，将需要默认启动拉起的进程添加到`Requires`字段中。建议将`Requires="sysinit.target"`更改为`Requires="sysinit.target;fstab.service;loopback-up.service;sshd.service;udevd.service;setip.service;getty-tty1.service;serial-getty-ttyAMA0.service"`。

**注意：**

- 上述配置中`serial-getty-ttyAMA0.service`服务是实现aarch64架构平台串口登陆所需的服务；如果是x86_64，那么需要将服务改为`serial-getty-ttyS0.service`，此服务主要针对有console串口的情况，例如`virsh console`进入串口，私人笔记本创建的虚拟机，默认应该都是只有tty1。
- 需要根据各自环境修改`setip.service`内点`ExecStart`的**ip地址及网关**；这里的ip地址可以设置为当前虚拟机的ip地址，可通过ip a获取；网关可以通过`netstat -nr`查询。这里设置成功虚拟机启动后即可以通过ssh登陆环境。

**5.** 由于sysmaster日志默认输出到串口，会影响串口登陆，同时sysmaster日志也无法详细查看。建议配置`/etc/sysmaster/system.toml`，将日志输出到本地文件中。

```
cat /etc/sysmaster/system.toml
LogLevel="debug"
LogTarget="file"
LogFile="/sysmaster.log"
```

至此，大系统准备完毕。



## 启动

虚拟机重启，在grub引导启动界面选择对于的启动项。启动后，可以通过tty1或者对于的console串口登陆。或者通过setip.services设置的ip进行ssh登陆。注意，通过tty1或者串口登陆，由于当前对于的getty服务自动重启有bug，所以只能输入一次账户和密码，输入错误则无法再次出现登陆界面。
