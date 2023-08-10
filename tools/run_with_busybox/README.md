# 兼容busybox模式运行

## 思路

以sysmaster为1号进程，拉起busybox初始化脚本，如果有业务进程，也可并行启动，加快开机进程。

[详细步骤请查看](http://sysmaster.online/resolution/00-systemd2sysmaster/)
