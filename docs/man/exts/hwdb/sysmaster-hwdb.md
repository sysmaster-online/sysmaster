# sysmaster-hwdb 使用手册

## 1. 简介
sysmaster-hwdb 是硬件数据库(hwdb)的管理工具，主要用于管理二进制格式的硬件数据库 hwdb.bin：生成二进制格式的硬件数据库、二进制格式的硬件数据库数据查询。

## 2. 大纲
sysmaster-hwdb的命令主要分为以下两种：
```
sysmaster-hwdb update [OPTIONS]
sysmaster-hwdb query [OPTIONS] <MODALIAS>
```

## 3. 选项
### sysmaster-hwdb update [OPTIONS]
更新二进制格式的硬件数据库。
```
--usr
    输出到 /usr/lib/devmaster 目录中(而不是默认的 /etc/devmaster 目录)。

-r, --root=PATH
    指定根文件系统的路径。

-s, --strict
    在更新时，如果遇到任何解析错误，那么就返回非零退出码表示出错。

-h, --help
    显示简短的帮助信息并退出。
```

### sysmaster-hwdb query [OPTIONS] <MODALIAS\>
查询二进制格式的硬件数据库，并显示查询结果。
```
-r, --root=PATH
    指定根文件系统的路径。

-h, --help
    显示简短的帮助信息并退出。
```
