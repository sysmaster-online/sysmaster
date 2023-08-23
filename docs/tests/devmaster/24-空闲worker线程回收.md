# 24 空闲worker线程回收

## 特性描述

事件队列清空后，若3s内无新设备事件输入，则定时器事件触发并回收所有worker

## 特性约束

无

## 子场景

测试空闲worker线程回收功能

### 备注

1、devmaster的配置文件如下
```
# cat /etc/devmaster/config.toml
rules_d = ["/tmp/test/rules.d"]
log_level = "info"
```

2、测试使用的sda、sr0、ens33设备仅限于本机，观测点的b8\:0、b11\:0是sda和sr0设备对应的名称，根据自己的环境自行替换

3、关于时间戳、进程号信息，具体根据自己的回显结果为准，表格仅显示当时测试的结果

### 准备工作

重启devmaster并获取进程号：4835

### 测试步骤

查看devmaster的线程数，有预期结果1
```
# ps -T -p 4835 | grep devmaster
```

触发全量设备事件，并查看devmaster的线程数，有预期结果2
```
# devctl trigger
# ps -T -p 4835 | grep devmaster
```

等待10s，再次查看devmaster的线程数，有预期结果1（本机测试大概6秒后就会结束，以实际测试环境为准设置一个大概的时间，确保事件处理完且后续3秒内无新的事件）
```
# sleep 10
# ps -T -p 4835 | grep devmaster
```

### 结果验证

预期结果1：只有1行devmaster信息（主进程）
预期结果2：有4行devmaster信息（1个主进程+3个worker线程，max_workers默认配置为3）

### 测试结束

无

### 测试场景约束

无
