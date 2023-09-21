# 使用sysMaster特权容器

## 什么是特权容器？

特权容器是一种特殊类型的容器，拥有主机上的所有功能，它解除了普通容器的所有限制。这意味着特权容器几乎可以做主机上可操作的所有行为，包括修改容器的内核参数等。

在K8s中，特权容器可以获得额外的特权，例如能够访问主机的文件系统、网络和进程等。攻击者如果获得了对特权容器的访问权，或者拥有创建新的特权容器的权限，就可以获得对主机资源的访问权，甚至可以创建一个高权限的绑定角色，从而对整个集群进行控制。

因此，在创建和使用特权容器时，需要特别注意安全风险，并采取必要的安全措施来保护主机和集群的安全。

## 什么是sysMaster特权容器？

sysMaster特权容器是指，以sysMaster为1号进程，进而在容器中拉起相关的系统服务，并且具备privileged特权的容器。用户可通过ssh登录到特权容器后通过nsenter获取特权容器所在的HostOS的root shell。可应用于云或集群场景下的HostOS运维，增强运维效率和提升整个系统的安全性。

![image](assets/admin_docker.png)

功能特点：

1. 容器化运维，特权模式，可陷入HOST；
2. 特权容器可通过K8S按需调度，灵活易用，即用即走；
3. 进一步精简HostOS，转而在特权容器中部署更多运维和调测工具；
4. HostOS可不提供登录通道，更加安全。



## 如何制作特权容器？

**1、通过Dockfile制作特权容器**

```dockerfile
FROM openeuler-22.03-lts:latest
RUN yum -y install openssh-server KubeOS-admin-container sysmaster
CMD ["/usr/lib/sysmaster/init"]
```

**2、通过kubectl部署特权容器**

特权容器部署示例YAML请见**特权容器部署YAML示例**，假定YAML保存到当前目录的admin-container.yaml，指定部署命令：

```bash
kubectl apply -f admin-container.yaml
```

部署完成后通过以下命令行查看特权容器是否正常启动，如果STATUS都是Running的，说明正常启动了。

```bash
kubectl get pods -A
```

**admin-container.yaml示例**如下：

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: root-secret
data:
  ssh-pub-key: your-ssh-pub-key
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: admin-container-sysmaster
  namespace: default
  labels:
      control-plane: admin-container-sysmaster
spec:
  selector:
    matchLabels:
      control-plane: admin-container-sysmaster
  replicas: 1
  template:
    metadata:
      labels:
        control-plane: admin-container-sysmaster
    spec:
      hostPID: true
      containers:
      - name: admin-container-sysmaster
        image: your_imageRepository/admin_imageName:version
        imagePullPolicy: Always
        securityContext:
          privileged: true
        ports:
          - containerPort: 22
        # sysmaster要求
        env:
          - name: container
            value: containerd
        volumeMounts:
        # name 必须与下面的卷名匹配
        - name: secret-volume
        # mountPath必须为/etc/secret-volume
          mountPath: /etc/secret-volume
          readOnly: true
      nodeName: your-worker-node-name
      volumes:
        - name: secret-volume
          secret:
            # secretName必须与上面指定的Secret的name相同
            secretName: root-secret
---
apiVersion: v1
kind: Service
metadata:
  name: admin-container-sysmaster
  namespace: default
spec:
  type: NodePort
  ports:
    - port: 22
      targetPort: 22
      nodePort: your-exposed-port
  selector:
      control-plane: admin-container-sysmaster
```

## 特权容器使用指导

特权容器部署后用户可通过ssh免密登录节点的特权容器，进入特权容器后执行hostshell命令获取节点的root shell。

root用户ssh登录倒特权容器上，其中，your-exposed-port必须和部署YAML中Service设置的nodePort映射端口保持一致：

```
ssh -p your-exposed-port root@your.worker.node.ip
```

登录后执行hostshell。

```
hostshell
```

## 使用约束

- hostshell需要root用户使用。
- 特权容器内1号进程为sysmaster，不支持systemd。
- 特权容器设计为运维场景使用的特殊容器，ssh登录特权容器需为root用户。
- 在host上使用特权容器内命令时，若该命令会使用固定路径下的文件，例如/etc/xxxx，而host上该文件不存在，可能会出现命令执行失败。
- 特权容器部署时需要为特权容器，并和host共享命名空间。
- 特权容器设计为运维场景使用的特殊容器，特权容器部署的权限由k8s控制，特权容器部署前k8s需对当前用户进行认证和鉴权。
