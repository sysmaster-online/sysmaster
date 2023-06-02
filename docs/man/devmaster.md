# devmaster 用户手册

## 配置文件

### 配置文件路径

`/etc/devmaster/config.toml`

### 配置选项

`rules_d`: 规则加载路径，默认值为`["/etc/devmaster/rules.d", "/run/devmaster/rules.d", "/usr/local/lib/devmaster/rules.d", "/usr/lib/devmaster/rules.d"]`。

`children_max`: 最大worker线程并发数，默认为3。

`log_level`: 日志级别，支持`"trace"`，`"debug"`，`"info"`，`"warn"`，`"error"`，`"off"`，默认值为`"info"`。
