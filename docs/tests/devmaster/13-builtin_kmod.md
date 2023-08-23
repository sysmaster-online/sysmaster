# 13 内置命令 kmod

## 特性描述

kmod内置命令，用于加载ko

## 特性约束

无

## 子场景 1

加载cls_bpf内核模块

### 备注

无

### 准备工作

无

### 测试步骤

步骤1：检查环境中未加载cls_bpf内核模块
```
# lsmod | grep cls_bpf
```

步骤2：使用devmaster的kmod内置命令，加载cls_bpf
```
# devctl test-builtin "kmod load cls_bpf" /sys/class/block/sda1
```

步骤3：检查环境中是否已加载cls_bpf
```
# lsmod | grep cls_bpf
```
### 结果验证

步骤3可以搜索到cls_bpf的信息
```
# lsmod | grep cls_bpf
cls_bpf 24576 0
```

### 测试结束

从环境中卸载cls_bpf内核模块
```
# rmmod cls_bpf
```

### 测试场景约束

系统中包含cls_bpf内核模块，否则使用其他内核模块代替。
