# 14 内置命令 net_id

## 特性描述

net_id内置命令，用于读取网卡属性

## 特性约束

无

## 子场景 1

测试读取网卡属性

### 备注

无

### 准备工作

无

### 测试步骤

步骤1：使用devmaster的net_id内置命令获取网卡设备的属性
```
# devctl test-builtin net_id /sys/class/net/<网卡设备>
```

步骤2：使用udev的net_id内置命令获取网卡设备的属性
```
# udevadm test-builtin net_id /sys/class/net/<网卡设备>
```

### 结果验证

预期结果:步骤1和2中如下项应保持一致，不在此列表中的值不比较，
```
ID_NET_NAME_MAC
ID_NET_NAME_ONBOARD
ID_NET_NAME_PATH
ID_NET_NAME_SLOT
ID_NET_NAME_PATH
```

```
# udevadm test-builtin net_id /sys/class/net/ens33/
ID_NET_NAMING_SCHEME=v249
ID_NET_NAME_MAC=enx000c297d049e
ID_OUI_FROM_DATABASE=VMware, Inc.
ID_NET_NAME_PATH=enp2s1
ID_NET_NAME_SLOT=ens33

# devctl test-builtin net_id /sys/class/net/ens33/
ID_NET_NAMING_SCHEME=latest
ID_NET_NAME_MAC=enx000c297d049e
ID_NET_NAME_PATH=enp2s1
ID_NET_NAME_SLOT=ens33
```

### 测试结束

无

### 测试场景约束

无
