# 02 客户端命令 devctl trigger

## 特性描述

触发设备事件

## 特性约束

触发设备事件

## 子场景 1

测试devctl trigger是否可以正常触发动作

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

1、触发事件
```
# devctl trigger  --action=[add/remove/change/move/online/offline/bind/unbind] $loopnum
```

2、新开窗口执行udevadm monitor监听事件

### 结果验证

udevadm monitor可以监听到对应事件,证明触发成功

```
KERNEL[9328.097468] remove   /devices/virtual/block/loop0 (block)
UDEV  [9328.099162] remove   /devices/virtual/block/loop0 (block)
KERNEL[9344.398764] add      /devices/virtual/block/loop0 (block)
UDEV  [9344.426996] add      /devices/virtual/block/loop0 (block)
KERNEL[9352.495032] change   /devices/virtual/block/loop0 (block)
UDEV  [9352.510515] change   /devices/virtual/block/loop0 (block)
KERNEL[9371.823544] move     /devices/virtual/block/loop0 (block)
UDEV  [9371.837565] move     /devices/virtual/block/loop0 (block)
KERNEL[9378.787949] online   /devices/virtual/block/loop0 (block)
UDEV  [9378.798706] online   /devices/virtual/block/loop0 (block)
KERNEL[9384.258377] offline  /devices/virtual/block/loop0 (block)
UDEV  [9384.274765] offline  /devices/virtual/block/loop0 (block)
KERNEL[9394.125083] bind     /devices/virtual/block/loop0 (block)
UDEV  [9394.141505] bind     /devices/virtual/block/loop0 (block)
KERNEL[9399.676206] unbind   /devices/virtual/block/loop0 (block)
UDEV  [9399.698422] unbind   /devices/virtual/block/loop0 (block)
```

### 测试结束

```
# losetup -d $loopnum
# rm -rf loop.img
```

### 测试场景约束

无
