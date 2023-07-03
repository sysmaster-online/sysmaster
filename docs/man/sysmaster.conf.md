# sysmaster 系统配置

sysmaster支持从`/etc/sysmaster/system.conf`中读取系统配置，用于配置`sysmaster`的日志。

## 日志配置

支持通过`LogLevel`、`LogTarget`分别配置日志的输出级别、输出目标。在配置`LogTarget="file"`时，允许通过`LogFileSize`，`LogFileNumber`配置转储日志文件的大小及数量。

**注意：**日志相关的配置为字符串，不能省略双引号。

### LogLevel

* 类型：字符串

支持配置为`"trace"`，`"debug"`，`"info"`，`"warn"`，`"error"`，`"off"`。缺省值为`"debug"`。

### LogTarget

* 类型：字符串

支持配置为`"syslog"`，`"console-syslog"`，`"file"`，缺省值为`"syslog"`。

* 配置为`"syslog"`，日志将输出到系统日志。
* 配置为`"console-syslog"`，日志将在输出到系统日志的基础上，同时打印到终端。
* 配置为`"file"`，日志将输出到`/var/log/sysmaster/sysmaster.log`，文件权限为600，`/var/log/sysmaster`目录的权限为700。

### LogFileSize

* 类型：数值，单位（kB）

配置`LogTarget`为`"file"`时，sysmaster会将日志生成到`/var/log/sysmaster/sysmaster.log`，如果`sysmaster.log`的大小超过`LogFileSize`的配置，将进行日志转储，转储的文件名`sysmaster.log.1`，`sysmaster.log.2`。数字越小表示日志文件越新。

**注意：** 这里的转储门限并不是精确的遵从用户配置，为避免日志被截断，会有微小浮动。

### LogFileNumber

* 类型：数值

配置转储文件的数量，当文件数量超限时，会自动删除最旧的日志文件。

## 外置db配置

支持通过`DbSize`等配置调整外置db参量。

### DbSize

* 类型：数值

DbSize参量支持配置为最大内存占用规格，单位为字节。当配置值小于当前sysmaster所用内存值时，以当前sysmaster所用内存值为准。此配置在系统启动或者daemon-reexec后生效。
