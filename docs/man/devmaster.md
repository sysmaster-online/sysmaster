# devmaster 用户手册

## 配置文件

### 配置文件路径

`/etc/devmaster/config.toml`

### 配置选项

`rules_d`: 规则加载路径，默认值为`["/etc/devmaster/rules.d", "/run/devmaster/rules.d", "/usr/local/lib/devmaster/rules.d", "/usr/lib/devmaster/rules.d"]`。

`max_workers`: 最大worker线程并发数，默认为3。

`log_level`: 日志级别，支持`"trace"`，`"debug"`，`"info"`，`"warn"`，`"error"`，`"off"`，默认值为`"info"`。

### 网卡重命名策略设置

当前支持的网卡重命名策略包含`v023`和`latest`，`latest`采用最新版本的网卡命名策略，新版本策略覆盖老版本策略。设置为`v000`或`0`关闭网卡重命名规则。

1. 配置启动参数: `net.naming-scheme=<scheme>`

2. 设置环境变量: `NET_NAMING_SCHEME=<scheme>`

NOTES: 环境变量`NET_NAMING_SCHEME`的值以':'开头时，则优先使用启动参数配置的策略，否则优先使用环境变量配置的策略。
