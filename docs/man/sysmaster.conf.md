# sysmaster 系统配置

sysmaster支持从`/etc/sysmaster/system.toml`中读取系统配置，用于配置`sysmaster`的日志。

## 日志配置

支持通过`LogLevel`、`LogTarget`、`LogFile`分别配置日志的输出级别、输出目标即输出路径，`LogFile`的配置只有当`LogTarget`配置为`file`时生效。

**注意：**日志相关的配置为字符串，不能省略双引号。

### LogLevel

* 类型：字符串

支持配置为`"trace"`，`"debug"`，`"info"`，`"warn"`，`"error"`，`"off"`。缺省值为`"debug"`。

### LogTarget

* 类型：字符串

支持配置为`"console"`，`"file"`，`"syslog"`。配置为`"console"`，日志将输出到终端；配置为`"file"`，日志将输出到`LogFile`配置的路径，此时`LogFile`必须配置为合法的绝对地址；配置为`"syslog"`，日志将输出到系统日志。缺省值为`"console"`。

### LogFile

* 类型：字符串

支持配置为`"`括起来的绝对路径，仅当`LogTarget`配置为`"file"`时生效。如果配置为空或不配置，将强制修改`LogTarget`为`"console"`。

### LogFileSize

* 类型：数值

配置`LogTarget`为`"file"`时，sysmaster支持对生成的日志进行转储，转储的文件名为在`LogFile`的配置基础上追加数字。`LogFileSize`配置转储门限，单位为`kB`。例如配置`LogTarget="file", LogFile="/var/log/sysmaster.log", LogFileSize=1`，sysmaster将自动将日志生成到`/var/log/sysmaster.log`，当`sysmaster.log`大小超过1kB时，将自动转储为`sysmaster.log.0`。

**注意：** 这里的转储门限并不是精确的遵从用户配置，为避免日志被截断，会有微小浮动。

### LogFileNumber

* 类型：数值

配置转储文件的数量，当文件数量超限时，会自动删除最旧的日志文件。

## 外置db配置

支持通过`DbSize`等配置调整外置db参量。

### DbSize

* 类型：数值

DbSize参量支持配置为最大内存占用规格，单位为字节。当配置值小于当前sysmaster所用内存值时，以当前sysmaster所用内存值为准。此配置在系统启动或者daemon-reexec后生效。
