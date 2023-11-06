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

- `devmaster`的规则直接取自虚拟机环境中的`udev`规则，在此基础上对`sysmaster`环境中不支持的规则进行了适配。引用的规则以及适配情况见`规则清单`和`适配差异`。

## 安装方法

1. 参考[`sysmaster`虚拟机启动方案](<http://127.0.0.1:8000/use/%E8%99%9A%E6%9C%BA%E4%B8%AD%E6%9B%BF%E4%BB%A3pid1%E8%BF%90%E8%A1%8C/readme/>)搭建启动环境。

2. 安装`devmaster`的二进制以及相关配置文件，在`sysmaster`源码根目录下执行如下命令：

   ```shell
   # sh -x tools/run_with_devmaster/install_devmaster.sh [debug|release]
   ```

3. 重启系统，选择`sysmaster`启动项。进入主系统后检查`devmaster`服务状态和网络连通情况。

## 规则清单

|编号  |  规则 | 包  | 是否需要适配 |
| ---------------- | --- |-|-|
|1  | 01-md-raid-creating.rules            | mdadm-4.1-5.oe2203.x86_64              | |
|2  | 10-dm.rules                          | device-mapper-1.02.181-4.oe2203.x86_64 | |
|3  | 11-dm-lvm.rules                      | lvm2-2.03.14-4.oe2203.x86_64           | |
|4  | 11-dm-mpath.rules                    | multipath-tools-0.8.7-2.oe2203.x86_64  | |
|5  | 11-dm-parts.rules                    | kpartx-0.8.7-2.oe2203.x86_64           | |
|6  | 13-dm-disk.rules                     | device-mapper-1.02.181-4.oe2203.x86_64 | |
|7  | 40-elevator.rules                    | systemd-udev-249-16.oe2203.x86_64      | |
|8  | 40-openEuler.rules                   | systemd-udev-249-16.oe2203.x86_64      | 是 |
|9  | 50-udev-default.rules                | systemd-udev-249-16.oe2203.x86_64      | |
|10 | 60-autosuspend.rules                 | systemd-udev-249-16.oe2203.x86_64      | |
|11 | 60-block.rules                       | systemd-udev-249-16.oe2203.x86_64      | |
|12 | 60-cdrom_id.rules                    | systemd-udev-249-16.oe2203.x86_64      | |
|13 | 60-drm.rules                         | systemd-udev-249-16.oe2203.x86_64      | |
|14 | 60-evdev.rules                       | systemd-udev-249-16.oe2203.x86_64      | |
|15 | 60-fido-id.rules                     | systemd-udev-249-16.oe2203.x86_64      | |
|16 | 60-infiniband.rules                  | none package                           | |
|17 | 60-input-id.rules                    | systemd-udev-249-16.oe2203.x86_64      | |
|18 | 60-net.rules                         | initscripts-10.12-1.oe2203.x86_64      | |
|19 | 60-persistent-alsa.rules             | systemd-udev-249-16.oe2203.x86_64      | |
|20 | 60-persistent-input.rules            | systemd-udev-249-16.oe2203.x86_64      | |
|21 | 60-persistent-storage.rules          | systemd-udev-249-16.oe2203.x86_64      | |
|22 | 60-persistent-storage-tape.rules     | systemd-udev-249-16.oe2203.x86_64      | |
|23 | 60-persistent-v4l.rules              | systemd-udev-249-16.oe2203.x86_64      | |
|24 | 60-raw.rules                         | util-linux-2.37.2-5.oe2203.x86_64      | |
|25 | 60-rdma-ndd.rules                    | rdma-core-35.1-2.oe2203.x86_64         | |
|26 | 60-rdma-persistent-naming.rules      | rdma-core-35.1-2.oe2203.x86_64         | |
|27 | 60-sensor.rules                      | systemd-udev-249-16.oe2203.x86_64      | |
|28 | 60-serial.rules                      | systemd-udev-249-16.oe2203.x86_64      | |
|29 | 60-srp_daemon.rules                  | rdma-core-35.1-2.oe2203.x86_64         | 是 |
|30 | 62-multipath.rules                   | multipath-tools-0.8.7-2.oe2203.x86_64  | 是 |
|31 | 63-md-raid-arrays.rules              | mdadm-4.1-5.oe2203.x86_64              | |
|32 | 64-btrfs-dm.rules                    | btrfs-progs-5.15-1.oe2203.x86_64       | |
|33 | 64-btrfs.rules                       | systemd-udev-249-16.oe2203.x86_64      | |
|34 | 64-md-raid-assembly.rules            | mdadm-4.1-5.oe2203.x86_64              | |
|35 | 66-kpartx.rules                      | kpartx-0.8.7-2.oe2203.x86_64           | |
|36 | 68-del-part-nodes.rules              | kpartx-0.8.7-2.oe2203.x86_64           | |
|37 | 69-dm-lvm.rules                      | lvm2-2.03.14-4.oe2203.x86_64           | 是 |
|38 | 69-md-clustered-confirm-device.rules | mdadm-4.1-5.oe2203.x86_64              | |
|39 | 70-camera.rules                      | none package                           | |
|40 | 70-joystick.rules                    | systemd-udev-249-16.oe2203.x86_64      | |
|41 | 70-memory.rules                      | systemd-249-16.oe2203.x86_64           | |
|42 | 70-mouse.rules                       | systemd-udev-249-16.oe2203.x86_64      | |
|43 | 70-power-switch.rules                | systemd-udev-249-16.oe2203.x86_64      | |
|44 | 70-touchpad.rules                    | systemd-udev-249-16.oe2203.x86_64      | |
|45 | 70-uaccess.rules                     | systemd-udev-249-16.oe2203.x86_64      | |
|46 | 71-seat.rules                        | systemd-udev-249-16.oe2203.x86_64      | |
|47 | 73-idrac.rules                       | systemd-udev-249-16.oe2203.x86_64      | |
|48 | 73-seat-late.rules                   | systemd-udev-249-16.oe2203.x86_64      | |
|49 | 75-net-description.rules             | systemd-udev-249-16.oe2203.x86_64      | |
|50 | 75-probe_mtd.rules                   | systemd-udev-249-16.oe2203.x86_64      | |
|51 | 75-rdma-description.rules            | rdma-core-35.1-2.oe2203.x86_64         | |
|52 | 78-sound-card.rules                  | systemd-udev-249-16.oe2203.x86_64      | |
|53 | 80-drivers.rules                     | systemd-udev-249-16.oe2203.x86_64      | |
|54 | 80-net-setup-link.rules              | systemd-udev-249-16.oe2203.x86_64      | |
|55 | 80-tpm-udev.rules                    | tpm2-tss-3.1.0-1.oe2203.x86_64         | |
|56 | 80-udisks2.rules                     | udisks2-2.9.4-2.oe2203.x86_64          | |
|57 | 81-net-dhcp.rules                    | systemd-249-16.oe2203.x86_64           | |
|58 | 84-nm-drivers.rules                  | NetworkManager-1.32.12-8.oe2203.x86_64 | |
|59 | 85-nm-unmanaged.rules                | NetworkManager-1.32.12-8.oe2203.x86_64 | |
|60 | 90-iprutils.rules                    | iprutils-2.4.19-1.oe2203.x86_64        | |
|61 | 90-iwpmd.rules                       | rdma-core-35.1-2.oe2203.x86_64         | |
|62 | 90-nm-thunderbolt.rules              | NetworkManager-1.32.12-8.oe2203.x86_64 | |
|63 | 90-rdma-hw-modules.rules             | rdma-core-35.1-2.oe2203.x86_64         | |
|64 | 90-rdma-ulp-modules.rules            | rdma-core-35.1-2.oe2203.x86_64         | |
|65 | 90-rdma-umad.rules                   | rdma-core-35.1-2.oe2203.x86_64         | |
|66 | 90-vconsole.rules                    | systemd-udev-249-16.oe2203.x86_64      | 是 |
|67 | 91-drm-modeset.rules                 | libdrm-2.4.109-2.oe2203.x86_64         | |
|68 | 95-dm-notify.rules                   | device-mapper-1.02.181-4.oe2203.x86_64 | |
|69 | 96-e2scrub.rules                     | e2fsprogs-1.46.4-7.oe2203.x86_64       | |
|70 | 98-kexec.rules                       | kexec-tools-2.0.23-4.oe2203.x86_64     | 是 |
|71 | 99-systemd.rules                     | systemd-udev-249-16.oe2203.x86_64      | 是 |

**`systemd`相关命令无法在`sysmaster`启动环境中使用，使用到这些命令的规则需要进行适配。部分适配动作会对功能造成影响，比如注释某条规则。某些适配动作不影响功能，但会产生行为差异，比如规则中调用`systemd-run <cmd>`命令会生成一个`systemd`的服务并执行`cmd`命令，适配后会去除`systemd-run`前缀，直接由`devmaster`创建子进程来执行`cmd`。另外清理了规则中的一些语法告警，比如`IMPORT{}`类规则使用了赋值操作符等，此类修改不影响规则执行结果。**

## 规则适配差异

### 40-openEuler.rules

```shell
# diff tools/run_with_devmaster/rules.d/40-openEuler.rules /lib/udev/rules.d/40-openEuler.rules
17c17
< # ACTION=="add", SUBSYSTEM=="module", KERNEL=="bridge", RUN+="/usr/lib/systemd/systemd-sysctl --prefix=/proc/sys/net/bridge"
---
> ACTION=="add", SUBSYSTEM=="module", KERNEL=="bridge", RUN+="/usr/lib/systemd/systemd-sysctl --prefix=/proc/sys/net/bridge"
```

### 60-srp_daemon.rules

```shell
# diff 60-srp_daemon.rules /lib/udev/rules.d/60-srp_daemon.rules
1c1
< #SUBSYSTEM=="infiniband_mad", KERNEL=="*umad*", PROGRAM=="/bin/systemctl show srp_daemon -p ActiveState", RESULT=="ActiveState=active", ENV{SYSTEMD_WANTS}+="srp_daemon_port@$attr{ibdev}:$attr{port}.service"
---
> SUBSYSTEM=="infiniband_mad", KERNEL=="*umad*", PROGRAM=="/bin/systemctl show srp_daemon -p ActiveState", RESULT=="ActiveState=active", ENV{SYSTEMD_WANTS}+="srp_daemon_port@$attr{ibdev}:$attr{port}.service"
```

### 62-multipath.rules

```shell
# diff 62-multipath.rules /lib/udev/rules.d/62-multipath.rules
74,75c74
< # RUN+="/usr/bin/systemd-run --unit=cancel-multipath-wait-$kernel --description 'cancel waiting for multipath siblings of $kernel' --no-block --timer-property DefaultDependencies=no --timer-property Conflicts=shutdown.target --timer-property Before=shutdown.target --timer-property AccuracySec=500ms --property DefaultDependencies=no --property Conflicts=shutdown.target --property Before=shutdown.target --property Wants=multipathd.service --property After=multipathd.service --on-active=$env{FIND_MULTIPATHS_WAIT_UNTIL} /usr/bin/udevadm trigger --action=add $sys$devpath"
< RUN+="/usr/bin/devctl trigger --action=add $sys$devpath"
---
> RUN+="/usr/bin/systemd-run --unit=cancel-multipath-wait-$kernel --description 'cancel waiting for multipath siblings of $kernel' --no-block --timer-property DefaultDependencies=no --timer-property Conflicts=shutdown.target --timer-property Before=shutdown.target --timer-property AccuracySec=500ms --property DefaultDependencies=no --property Conflicts=shutdown.target --property Before=shutdown.target --property Wants=multipathd.service --property After=multipathd.service --on-active=$env{FIND_MULTIPATHS_WAIT_UNTIL} /usr/bin/udevadm trigger --action=add $sys$devpath"
85,90c84,89
< IMPORT{db}=="FIND_MULTIPATHS_WAIT_CANCELLED"
< # ENV{FIND_MULTIPATHS_WAIT_CANCELLED}!="?*", \
< #     ENV{FIND_MULTIPATHS_WAIT_UNTIL}=="?*", \
< #     ENV{FIND_MULTIPATHS_WAIT_UNTIL}!="0", \
< #     ENV{FIND_MULTIPATHS_WAIT_CANCELLED}="1", \
< #     RUN+="/usr/bin/systemctl stop cancel-multipath-wait-$kernel.timer"
---
> IMPORT{db}="FIND_MULTIPATHS_WAIT_CANCELLED"
> ENV{FIND_MULTIPATHS_WAIT_CANCELLED}!="?*", \
>       ENV{FIND_MULTIPATHS_WAIT_UNTIL}=="?*", \
>       ENV{FIND_MULTIPATHS_WAIT_UNTIL}!="0", \
>       ENV{FIND_MULTIPATHS_WAIT_CANCELLED}="1", \
>       RUN+="/usr/bin/systemctl stop cancel-multipath-wait-$kernel.timer"
```

### 69-dm-lvm.rules

```shell
# diff 69-dm-lvm.rules /lib/udev/rules.d/69-dm-lvm.rules
82,84c82,83
< IMPORT{program}=="/usr/sbin/lvm pvscan --cache --listvg --checkcomplete --vgonline --udevoutput --journal=output $env{DEVNAME}"
< # ENV{LVM_VG_NAME_COMPLETE}=="?*", RUN+="/usr/bin/systemd-run -r --no-block --property DefaultDependencies=no --unit lvm-activate-$env{LVM_VG_NAME_COMPLETE} lvm vgchange -aay --nohints $env{LVM_VG_NAME_COMPLETE}"
< ENV{LVM_VG_NAME_COMPLETE}=="?*", RUN+="/usr/sbin/lvm vgchange -aay --nohints $env{LVM_VG_NAME_COMPLETE}"
---
> IMPORT{program}="/usr/sbin/lvm pvscan --cache --listvg --checkcomplete --vgonline --udevoutput --journal=output $env{DEVNAME}"
> ENV{LVM_VG_NAME_COMPLETE}=="?*", RUN+="/usr/bin/systemd-run -r --no-block --property DefaultDependencies=no --unit lvm-activate-$env{LVM_VG_NAME_COMPLETE} lvm vgchange -aay --nohints $env{LVM_VG_NAME_COMPLETE}"
```

### 98-kexec.rules

```shell
# diff 98-kexec.rules /lib/udev/rules.d/98-kexec.rules
14,16c14
< #RUN+="/bin/sh -c '/usr/bin/systemctl is-active kdump.service || exit 0; /usr/bin/systemd-run --quiet --no-block /usr/lib/udev/kdump-udev-throttler'"
<
< RUN+="/bin/sh -c '/usr/bin/systemctl is-active kdump.service || exit 0; /usr/lib/udev/kdump-udev-throttler'"
---
> RUN+="/bin/sh -c '/usr/bin/systemctl is-active kdump.service || exit 0; /usr/bin/systemd-run --quiet --no-block /usr/lib/udev/kdump-udev-throttler'"
```

### 90-vconsole.rules

```shell
# diff tools/run_with_devmaster/rules.d/90-vconsole.rules /lib/udev/rules.d/90-vconsole.rules
3c12
< # ACTION=="add", SUBSYSTEM=="vtconsole", KERNEL=="vtcon*", RUN+="/usr/lib/systemd/systemd-vconsole-setup"
---
> ACTION=="add", SUBSYSTEM=="vtconsole", KERNEL=="vtcon*", RUN+="/usr/lib/systemd/systemd-vconsole-setup"
```

### 99-systemd.rules

`99-systemd.rules`规则中的所有内容均被注释。
