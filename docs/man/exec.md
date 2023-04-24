# 命令执行环境

sysmaster的`socket`、`service`会拉起进程，该文档描述他们独有的针对进程上下文的相关配置。

## WorkingDiretory

* 类型：字符串

配置进程的工作目录。支持配置为绝对路径或`"~"`。配置为`"~"`，工作目录将解析为当前用户的home目录。支持在路径前添加`"-"`，表示忽略目录不存在的错误。

## RootDirectory

* 类型：字符串

配置进程的根目录，仅支持绝对路径。命令执行前，sysmaster会调用[chroot(2)](https://man7.org/linux/man-pages/man2/chroot.2.html)修改命令执行的根目录。
