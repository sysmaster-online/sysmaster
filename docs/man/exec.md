# 命令执行环境

sysmaster的`socket`、`service`会拉起进程，该文档描述他们独有的针对进程上下文的相关配置。

**注意：** 当前暂不支持`socket`中配置以下内容。

## WorkingDiretory

* 类型：字符串

配置进程的工作目录。支持配置为绝对路径或`"~"`。配置为`"~"`，工作目录将解析为当前用户的home目录。支持在路径前添加`"-"`，表示忽略目录不存在的错误。

## RootDirectory

* 类型：字符串

配置进程的根目录，仅支持绝对路径。命令执行前，sysmaster会调用[chroot(2)](https://man7.org/linux/man-pages/man2/chroot.2.html)修改命令执行的根目录。

## RuntimeDirectory/StateDirectory

* 类型：字符串

配置进程的运行时目录，仅支持相对路径。sysmaster会在启动服务时，在相应的目录（参考下表）下创建配置的运行时目录，如果服务同时配置了`User`，`Group`，会修改运行时目录的属组、属主。

| 配置 | 在哪儿创建运行时目录 |
|-|-|
| RuntimeDirectory | /run |
| StateDirectory | /var/lib |

## RuntimeDirectoryPreserve

* 类型：字符串

允许配置为`"yes"`，`"no"`，`"restart"`，默认值为`"no"`。配置为`"yes"`时，关闭服务时会保留`RuntimeDirectory`生成的目录。配置为`"no"`时，关闭服务时将删除该目录。配置为`"restart"`时，重启服务时（手动重启或Restart触发）会保留该目录。
