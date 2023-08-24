# 兼容busybox模式运行

## 思路

以sysmaster为1号进程，拉起busybox初始化脚本，如果有业务进程，也可并行启动，加快开机进程。

## 适配

以init.service拉起原本由busybox拉起的初始化脚本

## 验证

以虚拟机方式运行，主要分为镜像构建与镜像运行

### 一、虚拟机镜像构建

1、进入 build_image 目录

2、执行 sh build_image.sh 构建虚拟机镜像

完成后获得镜像文件/tmp/image/sysmasterwithbusybox.aarch64-1.0.tar.xz

### 二、运行镜像 （需要支持virsh命令的物理机）

1、编译好sysmaster，将其与步骤一编译好的镜像按照如下目录结构放置

2、修改run_image.sh中的IP，NETMASK，GATEWAY配置

3、修改sysmasterwithbusybox.xml中的kernel，initrd字段目录

4、执行sh run_image.sh sysmasterwithbusybox.aarch64-1.0.tar.xz即可启动该镜像

5、根据配置的ip，直接ssh连接该虚拟机

```
[root@localhost image]# tree run_image
run_image
├── extra
│   ├── bak.xml
│   ├── etc
│   │   └── sysmaster
│   │       ├── basic.target
│   │       └── init.service
│   ├── rcS
│   ├── start_sshd
│   ├── sysmasterwithbusybox.xml
│   └── usr
│       ├── bin
│       │   └── sctl
│       └── lib
│           └── sysmaster
│               ├── fstab
│               ├── init
│               ├── plugin
│               │   ├── libmount.so
│               │   ├── libservice.so
│               │   ├── libsocket.so
│               │   ├── libtarget.so
│               │   └── plugin.conf
│               ├── random_seed
│               ├── rc-local-generator
│               ├── sysmaster
│               └── sysmonitor
├── run_image.sh
└── sysmasterwithbusybox.aarch64-1.0.tar.xz
```
