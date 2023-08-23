# 01 客户端命令 devctl test-builtin

## 特性约束

无

## 特性描述

测试builtin命令

## 子场景 1

测试test-builtin命令可以正常执行8个内置命令

### 备注

无

### 准备工作

无

### 测试步骤

```
# devctl test-builtin [blkid|input_id|keyboard|net_id|path_id|usb_id|kmod|net_setup_link] <device_path>
```

### 结果验证

```
# devctl test-builtin blkid /sys/class/block/sda1/
Builtin command: 'blkid'
Device: '/sys/class/block/sda1/'
Action: 'change'
example builtin init
ID_FS_UUID=ca28c2be-2eb4-4a38-a80a-eb4f2798349c
ID_FS_UUID_ENC=ca28c2be-2eb4-4a38-a80a-eb4f2798349c
ID_FS_VERSION=1.0
2023-08-13 19:50:18 libdevmaster::builtin::blkid not match key: BLOCK_SIZE=4096
ID_FS_TYPE=ext4
ID_FS_USAGE=filesystem
ID_PART_ENTRY_SCHEME=dos
ID_PART_ENTRY_UUID=7487add7-01
ID_PART_ENTRY_TYPE=0x83
ID_PART_ENTRY_FLAGS=0x80
ID_PART_ENTRY_NUMBER=1
ID_PART_ENTRY_OFFSET=2048
ID_PART_ENTRY_SIZE=2097152
ID_PART_ENTRY_DISK=8:0
```

### 测试结束

无

### 测试场景约束

支持8个内置命令
