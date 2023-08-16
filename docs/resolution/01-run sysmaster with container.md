# 容器中管理sshd等服务

## 思路
以openEuler容器镜像为基础，制作以sysmaster为1号进程的容器镜像，在镜像中实现拉起sshd服务，以满足kubeos admin容器要求。

## 容器制作
1. 下载openeuler基础容器镜像：https://repo.openeuler.org/openEuler-22.03-LTS-SP1/docker_img/
2. docker load -i openEuler-docker.aarch64.tar.xz 载入容器镜像；
3. 构建sysmaster并将编译所得的整个debug目录拷贝到Dockerfile所在目录；
4. 执行docker build -t [imagenam:tagname](例如syy:withsysmasterv2) . --rm,完成后可以通过docker images查看生成的容器镜像。

## 验证sshd功能
1. 启动docker镜像:docker run -itd --privileged -v /sys/fs/cgroup:/sys/fs/cgroup:rw syy:withsysmasterv1 /sbin/init
2. 进入镜像：docker exec -it CONTAINERID /bin/bash
3. sctl start sshd.service启动sshd服务,通过sctl status sshd.service可以查看sshd状态，如果启动成功，此容器环境即可实现ssh登陆操作
4. 涉及到的sshd相关服务有下面五个单元。

```
sshd-keygen@ecdsa.service
sshd-keygen@ed25519.service
sshd-keygen@rsa.service
sshd-keygen.target
sshd.service
```
