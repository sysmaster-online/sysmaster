# 安装与部署

`sysmaster`可应用于容器和虚拟机，本文档将以 `aarch64`系统为例说明如何在各场景下进行安装与部署。

## 软件要求

* 操作系统：`openEuler 23.09`

## 硬件要求

* `x86_64`架构、`aarch64`架构

## 容器场景安装与部署

1. 安装 docker

   ```bash
   yum install -y docker
   systemctl restart docker
   ```

2. 加载基础容器镜像

   下载容器镜像

   ```bash
   wget https://repo.openeuler.org/openEuler-23.09/docker_img/aarch64/openEuler-docker.aarch64.tar.xz
   xz -d openEuler-docker.aarch64.tar.xz
   ```

   加载容器镜像

   ```bash
   docker load --input openEuler-docker.aarch64.tar
   ```

3. 构建容器

   创建 Dockerfile

   ```bash
   cat << EOF > Dockerfile
   FROM openeuler-23.09
   RUN yum install -y sysmaster
   CMD ["/usr/lib/sysmaster/init"]
   EOF
   ```

   构建容器

   ```bash
   docker build -t openeuler-23.09:latest .
   ```

4. 启动并进入容器

      启动容器

      ```bash
      docker run -itd --privileged openeuler-23.09:latest
      ```

      获取`CONTAINERID`

      ```bash
      docker ps
      ```

      使用上一步获取到`CONTAINERID`进入容器

      ```bash
      docker exec -it CONTAINERID /bin/bash
      ```

## 虚拟机场景安装与部署

1. `initramfs`镜像制作
   为了避免 `initrd`阶段 `systemd`的影响，需要制作一个剔除 `systemd`的 `initramfs`镜像，并以该镜像进入 `initrd`流程。使用如下命令：

   ```bash
   dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd" /boot/initrd_withoutsd.img
   ```

2. 新增启动项

   在 `grub.cfg`中增加新的启动项，`aarch64`下的路径为 `/boot/efi/EFI/openEuler/grub.cfg`，`x86_64`下的路径为 `/boot/grub2/grub.cfg`，拷贝一份原有启动项，并做以下几处修改：

    * `menuentry` 项修改启动项名称 `openEuler (6.4.0-5.0.0.13.oe23.09.aarch64) 23.09`为 `openEuler 23.09 withoutsd`
    * `linux` 项内核启动参数修改 `root=/dev/mapper/openeuler-root ro` 为 `root=/dev/mapper/openeuler-root rw`
    * `linux` 项内核启动参数修改 `plymouth`，如果环境上安装了 `plymouth`, 需要添加 `plymouth.enable=0` 禁用 `plymouth`
    * `linux` 项内核启动参数增加 `init=/usr/lib/sysmaster/init`
    * `initrd` 项修改为 `/initrd_withoutsd.img`

3. 安装 sysmaster

   ```bash
   yum install sysmaster
   ```

4. 重启后出现 `openEuler 23.09 withoutsd`启动项表示已成功配置，选择此启动项进入虚拟机
