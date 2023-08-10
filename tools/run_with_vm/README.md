# 虚拟机场景运行sysmaster

## 思路

以openEuler LTS 22.03版本为基础镜像创建虚拟机，通过dracut重做initrd，去除systemd影响；同时虚拟机中以sysmaster为init进程，实现以sysmaster为1号进程的虚拟机。

[详细步骤请查看](http://sysmaster.online/resolution/00-systemd2sysmaster/)
