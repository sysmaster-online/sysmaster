# sysmaster使用说明

本章主要通过一些实例来带领用户初步使用 `sysmaster`，例如:

* 如何创建 `service`服务单元配置文件。
* 如何管理单元服务，例如启动、停止、查看服务。

## 创建单元配置文件

用户可以在 `/usr/lib/sysmaster/system/`目录下创建单元配置文件。

### 单元配置文件的类型

当前 `sysmaster`支持 `target`、`socket`、`service`类型的单元配置文件。

* `target`：封装了一个由 `sysmaster`管理的启动目标，用于将多个单元集中到一个同步点。`sysmaster`提供不同阶段的 `target`单元，例如   `multi-user.target`代表系统已完成启动，用户可以依赖此目标，启动自己的服务。
* `socket`：封装了一个用于进程间通信的套接字 `socket`， 以支持基于套接字的启动。例如用户可以配置 `service`单元依赖此 `socket`，当此 `socket`有数据写入时，`sysmaster`会拉起对应的 `service`单元。
* `service`：封装了一个被 `sysmaster`监视与控制的进程。

### 单元配置文件的构成

单元配置文件通常由3块组成：

* `Unit`：单元的公共配置说明，如服务名称、描述、依赖关系等。
* `Install`：描述如何安装和启动服务。
* `Service`、`Socket`：各个单元类型的配置。

### 创建service单元配置

`sshd`服务被用来远程登录到服务器，并在远程终端上执行命令和操作。
使用如下配置项来创建一个 `sshd.service`服务单元配置。

```bash
[Unit]
Description="OpenSSH server daemon"
Documentation="man:sshd(8) man:sshd_config(5)"
After="sshd-keygen.target"
Wants="sshd-keygen.target"

[Service]
Type="notify"
EnvironmentFile="-/etc/sysconfig/sshd"
ExecStart="/usr/sbin/sshd -D $OPTIONS"
ExecReload="/bin/kill -HUP $MAINPID"
KillMode="process"
Restart="on-failure"
RestartSec=42

[Install]
WantedBy="multi-user.target"
```

以下是对单元配置文件中选项配置的说明，更多可以查阅[官方手册](https://openeuler-sysmaster.github.io/sysmaster/index_zh/)。

* `Description`：说明该 `unit`的主要功能。
* `Documentation`：说明该 `unit`的文档链接。
* `After`：配置同时启动的单元的先后顺序，`sshd.service`服务将在 `sshd-keygen.target`之后启动。
* `Wants`：配置一个单元对另一个单元的依赖，启动 `sshd.service`服务，将会自动启动 `sshd-keygen.target`。
* `Type`：配置 `sysmaster` 如何启动此服务，`notify`表明需要主进程启动完成后发送通知消息。
* `EnvironmentFile`：设置环境变量的文件读取路径。
* `ExecStart`：配置服务启动时执行的命令，启动 `sshd.service`服务会执行 `sshd`命令。
* `ExecReload`：配置重新加载 `sshd.service`的配置时执行的命令。
* `KillMode`：配置当需要停止服务进程时，杀死服务进程的方法，`process`表示只杀死主进程。
* `Restart`：配置服务不同情况下退出或终止，是否重新启动服务，`on-failure`表示当服务非正常退出时重新启动服务。
* `RestartSec`：配置当服务退出时，重新拉起服务的间隔时间。
* `WantedBy`：配置依赖当前 `sshd.service`服务的单元。

## 管理单元服务

`sctl`是 `sysmaster`的命令行工具，用于检查和控制 `sysmaster`服务端行为和各个服务的状态，它可以启动、停止、重启、检查系统服务。

### 启动服务

使用以下命令可以启动 `sshd`服务和运行 `ExecStart`所配置的命令。

```bash
sctl start sshd.service
```

### 停止服务

使用以下命令可以停止 `sshd`服务，杀死 `ExecStart`所运行的进程。

```bash
sctl stop sshd.service
```

### 重启服务

使用以下命令可以重启 `sshd`服务，该命令会先停止后启动服务。

```bash
sctl restart sshd.service
```

### 查看服务状态

使用以下命令可以查看服务 `sshd`运行状态，用户可以查看服务的状态来获取服务是否正常运行。

```bash
sctl status sshd.service
```
