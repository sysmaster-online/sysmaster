# oemake制作sysmaster作为1号进程的iso镜像

## 一、安装工具包

1、配置待制作镜像同版本的repo源

```
vi /etc/yum.repos.d/openEuler.repo
[openEuler]
name=openEuler
baseurl=https://repo.openeuler.org/openEuler-22.03-LTS-SP2/everything/aarch64/
enabled=1
gpgcheck=0
priority=1
```

2、安装oemker、createrepo

```
yum install oemaker createrepo
```

## 二、制作本地源

1、下载everything镜像到/root目录

2、挂载everything镜像到本地/root/iso

```
cd /root
mkdir iso
mount openEuler-22.03-LTS-SP2-everything-aarch64-dvd.iso iso
```

3、定制sysmaster软件包，在sysmaster的spec文件进行定制修改

3.1、在%install中新增安装95devmaster/init.sh和95devmaster/module-setup.sh文件：

```
mkdir -p %{buildroot}/usr/lib/dracut/modules.d/95devmaster
install -Dm0755 -t %{buildroot}/usr/lib/dracut/modules.d/95devmaster exts/devmaster/dracut_modules/95devmaster/init.sh
install -Dm0755 -t %{buildroot}/usr/lib/dracut/modules.d/95devmaster exts/devmaster/dracut_modules/95devmaster/module-setup.sh
```

3.2、在%files -n devmaster中打包95devmaster/init.sh和95devmaster/module-setup.sh文件

```
%dir /usr/lib/dracut/modules.d/95devmaster
/usr/lib/dracut/modules.d/95devmaster/*
```

3.3、编写posttrans脚本：

```
%posttrans
if [ -L "/usr/sbin/init" ]; then
  rm -rf /usr/sbin/init
fi
ln -s /usr/lib/sysmaster/init /usr/sbin/init

#iso安装所有包之后会重做小系统，删除这些目录，确保重做的小系统不包含systemd相关信息，等效于重新执行了：dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd rngd dbus-daemon dbus network-manager plymouth" /boot/initramfs-`uname -r`.img命令
rm -rf /usr/lib/dracut/modules.d/00systemd /usr/lib/dracut/modules.d/01systemd-initrd /usr/lib/dracut/modules.d/01systemd-networkd /usr/lib/dracut/modules.d/98dracut-systemd /usr/lib/dracut/modules.d/06rngd /usr/lib/dracut/modules.d/06dbus-daemon /usr/lib/dracut/modules.d/09dbus /usr/lib/dracut/modules.d/35network-manager /usr/lib/dracut/modules.d/50plymouth
```

4、将编译出来的sysmaster和devmaster包下载下来保存在/root/repo目录

5、 生成本地源

```
cd /root/repo
createrepo .
```

## 三、修改oemaker配置

oemaker安装后的脚本路径为“/opt/oemaker”，在/opt/oemaker/config/{arch}/normal.xml中的Core列表里新增sysmaster和devmaster
```
<packagereq type="mandatory">sysmaster</packagereq>
<packagereq type="mandatory">devmaster</packagereq>
```

## 四、制作iso镜像

1、 保证有3个可用的loop设备（虚拟机可以不用管，但容器场景可能需要此条件）

```
ls /dev/loop* -l
```

2、制作iso镜像

```
oemaker -t standard -p openEuler -v 22.03-LTS-SP2 -r '' -s "file:///root/iso file:///root/repo"
```
