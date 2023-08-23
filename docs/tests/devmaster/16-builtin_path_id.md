# 16 内置命令 path_id

## 特性描述

path_id内置命令，用于读取设备的路径信息

## 特性约束

无

## 子场景 1

测试读取设备的路径信息

### 备注

无

### 准备工作

无

### 测试步骤

步骤1：使用devmaster的path_id内置命令获取设备的路径信息
```
# devctl test-builtin path_id /sys/class/block/<设备名称>
```

步骤2：使用udev的path_id内置命令获取设备的路径信息
```
# udevadm test-builtin path_id /sys/class/block/<设备名称>
```

### 结果验证

预期结果:步骤1和2中如下项应保持一致，不在此列表中的值不比较，
```
ID_PATH
ID_PATH_TAG
ID_PATH_ATA_COMPAT
```

```
# devctl test-builtin path_id /sys/class/block/sda1
ID_PATH=pci-0000:00:10.0-scsi-0:0:0:0
ID_PATH_TAG=pci-0000_00_10_0-scsi-0_0_0_0

# udevadm test-builtin path_id /sys/class/block/sda1
ID_PATH=pci-0000:00:10.0-scsi-0:0:0:0
ID_PATH_TAG=pci-0000_00_10_0-scsi-0_0_0_0
```

### 测试结束

无

### 测试场景约束

无
