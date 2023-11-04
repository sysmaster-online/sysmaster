# 22 remove事件处理

## 特性描述

移除标签；删除数据库；软链接清理

## 特性约束

无

## 子场景 1

测试remove存在tag和软链接信息的设备

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

1、创建规则文件/tmp/test/rules.d/01-test.rules
```
KERNEL=="sda", TAG+="test_tag", SYMLINK+="test_link"
```

2、重启devmaster

### 测试步骤

触发sda的change事件
```
# devctl trigger /dev/sda
```

查看sda的数据库内容，有预期结果1
```
# cat /run/devmaster/data/b8\:0
```

查看/run/devmaster/tags/test_tag目录，有预期结果2
```
# ls /run/devmaster/tags/test_tag
```

查看软链接/dev/test_link，有预期结果3
```
# ls -l /dev/test_link
```

查看/run/devmaster/links/test_link目录，有预期结果4
```
# ls -l /run/devmaster/links/test_link
```

触发sda的remove事件
```
# devctl trigger -a remove /dev/sda
```

查看sda的数据库，有预期结果5
```
# ls /run/devmaster/data/b8\:0
```

查看/run/devmaster/tags/test_tag目录，有预期结果6
```
# ls /run/devmaster/tags/test_tag
```

查看软链接/dev/test_link，有预期结果5
```
# ls -l /dev/test_link
```

查看/run/devmaster/links/test_link目录，有预期结果6
```
# ls -l /run/devmaster/links/test_link
```

### 结果验证

预期结果1：存在“S:test_link”“G:test_tag”和“Q:test_tag”
预期结果2：存在b8:0文件
预期结果3：存在/dev/test_link -> sda软链接
预期结果4：存在b8:0 -> 0:/dev/sda软链接
预期结果5：打印错误包含关键字“No such file or directory”
预期结果6：不存在b8:0文件或软链接

### 测试结束

```
# rm -rf /run/devmaster/data/b8\:0
# rm -rf /run/devmaster/tags/test_tag
# rm -rf /dev/test_link /run/devmaster/links/test_link
# rm -rf /tmp/test/rules.d/01-test-tag.rules
```

### 测试场景约束

无
