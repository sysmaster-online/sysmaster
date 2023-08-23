# 03 客户端命令 devctl monitor

## 特性描述

监听内核和用户态的uevent事件

## 特性约束

无

## 子场景 1

测试devctl monitor是否可以正常监听uevent事件

### 备注

无

### 准备工作

创建loop设备
```
# dd if=/dev/zero of=loop.img bs=1M count=100
# loopnum=$(losetup -f)
# losetup  $loopnum loop.img
# mkfs.ext4 $loopnum
```

### 测试步骤

1. 触发事件
```
# udevadm trigger --action=[add/remove/change/move/online/offline/bind/unbind]
```

2. 新开窗口执行查看`devctl monitor`是否有对应事件消息打印

### 结果验证

devctl monitor可以监听到对应事件
```
KERNEL [] >> remove /devices/virtual/block/loop0 (block)
2023-08-13 19:32:39 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> add /devices/virtual/block/loop0 (block)
2023-08-13 19:32:49 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> change /devices/virtual/block/loop0 (block)
2023-08-13 19:32:59 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> move /devices/virtual/block/loop0 (block)
2023-08-13 19:33:14 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> offline /devices/virtual/block/loop0 (block)
2023-08-13 19:33:21 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> online /devices/virtual/block/loop0 (block)
2023-08-13 19:33:27 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> unbind /devices/virtual/block/loop0 (block)
2023-08-13 19:33:34 devctl::subcmds::devctl_monitor Device error: origin from udev
KERNEL [] >> bind /devices/virtual/block/loop0 (block)
2023-08-13 19:33:37 devctl::subcmds::devctl_monitor Device error: origin from udev
```

### 测试结束

```
# losetup -d $loopnum
# rm -rf loop.img
```

### 测试场景约束

无
