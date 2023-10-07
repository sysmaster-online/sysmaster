# sysMaster 功能测试

**测试内容：**

1. 基本功能：支持x86_64、aarch64；
2. 场景：支持容器、虚拟机场景

# 容器场景下测试

## 搭建测试环境

### 安装构建工具

下载仓库代码及安装工具链, 生成的二进制程序位于 `target`下.

```bash
# 下载仓库代码
git clone https://gitee.com/openeuler/sysmaster.git
cd sysmaster

# 安装编译及运行环境, 仓库提供有对应的脚本.
sh ci/00-pre.sh

# 执行构建和测试, 只支持debug模式
sh ci/01-pre-commit.sh

# 自行构建
cargo build --all --release
```

### 部署容器测试环境

如果需要使用本地构建的`sysmaster`进行测试，则使用以下步骤构建容器测试环境。

- 归档二进制，归档后的二进制位于 `target/install`目录下，用于构建容器镜像.

```bash
sh -x install.sh release
```

- 安装docker

```bash
yum install -y docker

systemctl restart docker
```

- 创建容器

此脚本使用本地编译的sysmaster来创建容器。

```bash
sh tests/setup_docker.sh
```

## 基本功能测试

### 测试内容

基于容器场景，在x86_64、aarch64两种架构上测试sysmaster功能

### 测试平台

- x86_64

| **工具/** **硬件** | **参数**                            |
| ------------------------------ | ----------------------------------------- |
| **设备类型**             | 2288H V5                                  |
| **CPU**                  | Intel(R) Xeon(R) Gold 5218R CPU @ 2.10GHz |
| **内存**                 | 8*16G                                     |
| **硬盘**                 | SAS3408                                   |
| **虚拟机操作系统**       | openEuler-22.03-LTS-SP1                   |
| **docker版本**           | 18.09.0                                   |
| **gcc版本**              | 10.3.1                                    |
| **musl-gcc版本**         | 1.2.3                                     |

- aarch64

| **工具/** **硬件** | **参数**              |
| ------------------------------ | --------------------------- |
| **设备类型**             | Taishan 200                 |
| **CPU**                  | Kunpeng920-6426 @ 2.6GHz *2 |
| **内存**                 | 251G                        |
| **硬盘**                 | 1.5T                        |
| **虚拟机操作系统**       | openEuler-22.03-LTS-SP1     |
| **docker版本**           | 18.09.0                     |
| **gcc版本**              | 10.3.1                      |
| **musl-gcc版本**         | 1.2.3                       |

### 测试项

通过执行`cargo test`命令运行对应的测试用例，首次执行测试用例会自动构建容器测试环境，无需手动构建。测试的是当前`repo`仓中的`sysmaster`，如果需要测试本地`sysmaster`，可以参照上文[部署容器测试环境](#部署容器测试环境)

#### docker_config_test_dependency_001测试项

- 测试内容
测试服务启动失败后执行的操作
- 配置项
`ConditionPathExists`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_dependency_001 --exact --nocapture --ignored
```

#### docker_config_test_dependency_003测试项

- 测试内容
测试服务启动失败后执行的操作
- 配置项
`OnFailure`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_dependency_003 --exact --nocapture --ignored
```

#### docker_config_test_dependency_004测试项

- 测试内容
测试服务启动成功后执行的操作
- 配置项
`OnSuccess`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_dependency_004 --exact --nocapture --ignored
```

#### docker_config_test_dependency_005测试项

- 测试内容
测试服务执行的顺序
- 配置项
`Before`
`After`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_dependency_005 --exact --nocapture --ignored
```

#### docker_config_test_action_001测试项

- 测试内容
测试服务失败后`Action`是否执行
- 配置项
`SuccessAction`
`FailureAction`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_action_001 --exact --nocapture --ignored
```

- 测试结果

#### docker_config_test_condition_001测试项

- 测试内容
测试服务的运行条件
- 配置项
`ConditionPathExists`
`AssertPathExists`
`ConditionPathIsReadWrite`
`AssertPathIsReadWrite`
`ConditionDirectoryNotEmpty`
`ConditionFileIsExecutable`
`ConditionPathExistsGlob`
`ConditionPathIsDirectory`
`ConditionPathIsMountPoint`
`ConditionPathIsSymbolicLink`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_condition_001 --exact --nocapture --ignored
```

- 测试结果

#### docker_config_test_condition_003测试项

- 测试内容
测试检测条件类 `Condition`
- 配置项
`ConditionCapability`
`ConditionKernelCommandLine`
`ConditionSecurity`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_condition_003 --exact --nocapture --ignored
```

- 测试结果

#### docker_config_test_action_001测试项

- 测试内容
- 配置项
`SuccessAction`
`FailureAction`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_action_001 --exact --nocapture --ignored
```

#### docker_config_test_timeout_001测试项

- 测试内容
- 配置项
`JobTimeoutSec`
`JobTimeoutAction`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_timeout_001 --exact --nocapture --ignored
```

#### docker_config_test_service_001测试项

- 测试内容
- 配置项
`Description`
`Documentation`
`RemainAfterExit`
`DefaultDependencies`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_service_001 --exact --nocapture --ignored
```

- 测试结果

#### docker_config_test_service_002测试项

- 测试内容
- 配置项
`Type=foring`
`PIDFile`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_service_002 --exact --nocapture --ignored
```

#### docker_config_test_service_004测试项

- 测试内容
- 配置项
`User`
`Group`
`UMask`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_service_004 --exact --nocapture --ignored
```

#### docker_config_test_kill_001测试项

- 测试内容
- 配置项
`KillMode`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_kill_001 --exact --nocapture --ignored
```

#### docker_config_test_listen_001测试项

- 测试内容
- 配置项
`ListenDatagram`
`ListenStream`
`ListenSequentialPacket`
`ListenNetlink`
- 测试步骤
执行以下命令

```bash
cargo test --test docker_config_test -- docker_config_test_listen_001 --exact --nocapture --ignored
```

- 测试结果

# 虚拟机场景下测试

## 搭建测试环境

在编译完成后，进入源码根目录，使用安装脚本install_sysmaster.sh将sysmaster的二进制文件、系统服务、配置文件等安装到系统中

```bash
# 安装sysmaster
sh -x tools/run_with_vm/install_sysmaster.sh release
```

## 基本功能测试

验证在虚拟机场景下sysmaster基本功能

### 测试项

#### 登录功能

- 测试内容
此测试项主要测试能否将sysmaster做为1号进程，并运行用户通过tty1及ssh进行登录
- 测试步骤

1. 参考<http://sysmaster.online/use/01-run%20sysmaster%20with%20vm/#sysmaster> 搭建环境
2. 重启虚拟机，并以新启动项运行
3. 切换至tty1登录，通过ssh登录

- 测试结果
可以成功登录

#### service_001

- 测试内容
- 配置项
`Description`
`Documentation`
`RemainAfterExit`
`DefaultDependencies`
- 测试步骤
执行以下命令

```bash
cargo test --test vm_config_test -- service_001 --exact --nocapture --ignored
```

- 测试结果
