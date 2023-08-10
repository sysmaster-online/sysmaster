# 兼容systemd模式运行

## 思路
以sysmaster为1号进程，拉起systemd以非1号运行，并负责监控systemd的运行状态。

[详细步骤请查看](http://sysmaster.online/resolution/00-systemd2sysmaster/)
