# 兼容systemd模式运行

## 思路
以sysmaster为1号进程，拉起systemd以非1号运行，并负责监控systemd的运行状态。

## 适配
基于openEuler 21.09构建systemd，需要适配systemd源码以支持非1号运行, 所有内容放在[tools/run_with_sd]()下。

适配的代码放在[tools/run_with_sd/patches](patches/)目录，适配的systemd版本是systemd-248.13。

本次适配用于原型验证，故相关代码的修改皆以简单原则，满足功能验证即可。

适配中，部分问题未解决，规避处理。

1. 运行systemd需要关闭selinux， /etc/selinux/config, enforcing to disabled
2. shutdown中需要增加信号处理代码，用以处理非1号执行时接受子进程信号。当前直接跳过，不做处理。
3. systemd从crash中恢复的适配，未考虑状态不可信的问题。因此构造crash的过程中，有概率systemd无法从crash中恢复。
4. sysmaster具备监控保活的功能, systemd未实现

## 验证
至少有两种方式验证。
### 容器方式（在容器中非1号进程运行systemd）
1. 使用[patches](patches/)目录下的适配代码编译systemd，并将输出的rpms，放在[systemd-dockerimg/rpms](systemd-dockerimg/rpms/)目录下。
2. 使用build.sh(systemd-dockerimg/build.sh)构建systemd的容器image。可以`chroot rootfs  /bin/bash`，通过`password`命令修改登录密码，然后执行`build.sh`更新镜像。
3. 修改仓库根目录下的Dockerfile，修改`FROM scratch`为`FROM systemd`, 去除`#RUN rm -f /sbin/init`注释 并在根目录下执行`./docker-run.sh /usr/lib/systemd/systemd`
4. 可另起窗口，执行`docker exec -it prun bash`进入容器。

### 虚拟机方式（在虚拟机中非1号进程运行systemd）
1. 安装21.09虚拟机镜像。
2. 禁用selinux，/etc/selinux/config, enforcing to disabled
3. 将systemd的rpms上传到虚拟机中，并执行`rpm -Fvh *.rpm`安装升级。
4. 将sysmaster中编译的init进程，替换到虚拟机/init, /sbin/init，建议先`rm /sbin/init`删除init软链接，否则会覆盖systemd程序。
5. 修改dracut，`/usr/lib/dracut/modules.d/00systemd/module-setup.sh`, 替换

```
ln_r "$systemdutildir"/systemd "/init"
ln_r "$systemdutildir"/systemd "/sbin/init"
```
为

```
inst_multiple -o \
    /init \
    /sbin/init
```
6. 执行`dracut -f`覆盖更新initrd，reboot重启验证。
