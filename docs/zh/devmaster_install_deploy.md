# 安装与部署

`devmaster`目前可应用于虚拟机环境，本章节介绍安装部署的规格约束以及操作流程。

## 软件要求

* 操作系统：`openEuler 23.09`

## 硬件要求

* `x86_64`架构、`aarch64`架构

## 安装部署流程

1. 执行如下命令，使用`yum`工具安装`sysmaster-devmaster`包：

    ```shell
    # yum install sysmaster-devmaster
    ```

2. 执行如下命令，创建默认规则文件`/etc/devmaster/rules.d/99-default.rules`和常驻进程的配置文件`/etc/devmaster/config.toml`：

    ```shell
    # mkdir -p /etc/devmaster/rules.d
    # mkdir -p /etc/devmaster/network.d
    # echo "TAG+=\"devmaster\"" > /etc/devmaster/rules.d/99-default.rules
    # cat << EOF > /etc/devmaster/config.toml
    log_level = "info"
    rules_d = ["/etc/devmaster/rules.d"]
    network_d = ["/etc/devmaster/network.d"]
    max_workers = 1
    log_targets = ["console"]
    EOF
    ```

3. 执行如下命令启动常驻进程`devmaster`，并将日志导出到`/tmp/devmaster.log`文件中：

    ```shell
    # /lib/devmaster/devmaster &>> /tmp/devmaster.log &
    ```

    > [!NOTE]说明
    > `devmaster`需要以 `root`权限启动，并且不能和 `udev`同时处于运行状态，启动 `devmaster`前需要停止`udev`服务。
   
    要停止`udev`服务，`sysmaster`启动环境下，执行以下命令：

    ```shell
    # sctl stop udevd.service udevd-control.socket udevd-kernel.socket
    ```

    要停止`udev`服务，`systemd`启动环境下，执行以下命令：

    ```shell
    # systemctl stop systemd-udevd.service systemd-udevd systemd-udevd-kernel.socket systemd-udevd-control.socket
    ```

4. 执行如下命令，使用 `devctl`工具触发设备事件：

    ```shell
    # devctl trigger
    ```

5. 查看 `/run/devmaster/data/`目录，如果生成设备数据库，则表示部署成功：

    ```shell
    # ll /run/devmaster/data/
    ```
