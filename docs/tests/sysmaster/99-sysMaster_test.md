# sysMaster 功能测试

**测试内容：**

1. 基本功能：支持x86_64、aarch64；
2. 场景：支持容器、虚拟机场景
# 容器场景下测试

## 搭建测试环境

###  安装构建工具

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

###  部署容器测试环境

> 注: 可通过 `cargo test`执行集成测试用例时会自动化部署，无需手动执行。

- 归档二进制，归档后的二进制位于 `target/install`目录下，用于构建容器镜像.

```bash
sh -x install.sh release
```

- 安装docker

```bash
yum install -y docker

systemctl restart docker
```

- 加载基础容器镜像

```bash
OS_VER="openEuler-22.03-LTS-SP1"

DOCKER_IMG_URL="https://mirrors.nju.edu.cn/openeuler/${OS_VER}/docker_img/$(arch)/"

DOCKER_TAR="openEuler-docker.$(arch).tar"

BASE_IMG="${OS_VER,,}"

SYSMST_BASE_IMG="sysmaster_base-${BASE_IMG}"

TMP_DIR="$(mktemp -d /tmp/test_XXXX)"

wget -P "${TMP_DIR}" "${DOCKER_IMG_URL}/${DOCKER_TAR}".xz

xz -d "${TMP_DIR}"/"${DOCKER_TAR}".xz

docker load --input "${TMP_DIR}"/"${DOCKER_TAR}"

docker images
```

- 构建测试镜像

```bash
pushd "${TMP_DIR}"

cat << EOF > Dockerfile

FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}

RUN rm -rf /etc/yum.repos.d && mkdir /etc/yum.repos.d
COPY yum.repos.d /etc/yum.repos.d/
RUN yum install -y util-linux shadow sudo passwd net-tools iproute nmap
COPY install/usr/bin/sctl /usr/bin/
RUN mkdir /usr/lib/sysmaster /etc/sysmaster /usr/lib/sysmaster/system
COPY install/usr/lib/sysmaster /usr/lib/sysmaster/
COPY install/etc/sysmaster /etc/sysmaster/
RUN sed -i '/LogTarget/ s/=.*/="console-syslog"/' /etc/sysmaster/system.conf
EOF

cat Dockerfile

docker build -t "${SYSMST_BASE_IMG}:latest" .

popd
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

可通过执行cargo test命令执行对应的测试用例

#### docker_config_test_action_001
- 测试内容
测试服务失败后Action是否执行
- 配置项
SuccessAction
FailureAction
- 测试步骤
执行以下命令
```bash
cargo test --test docker_config_test -- docker_config_test_action_001 --exact --nocapture --ignored
```
- 测试结果

####  docker_config_test_condition_001
- 测试内容
测试服务的运行条件
- 配置项
ConditionPathExists
AssertPathExists
ConditionPathIsReadWrite
AssertPathIsReadWrite
ConditionDirectoryNotEmpty
ConditionFileIsExecutable
ConditionPathExistsGlob
ConditionPathIsDirectory
ConditionPathIsMountPoint
ConditionPathIsSymbolicLink
- 测试步骤
执行以下命令
```
cargo test --test docker_config_test -- docker_config_test_condition_001 --exact --nocapture --ignored
```
- 测试结果

#### docker_config_test_condition_003
- 测试内容
测试检测条件类 Condition
- 配置项
ConditionCapability
ConditionKernelCommandLine
ConditionSecurity
- 测试步骤
执行以下命令
```
cargo test --test docker_config_test -- docker_config_test_condition_003 --exact --nocapture --ignored
```
- 测试结果

#### docker_config_test_service_001
- 测试内容
- 配置项
Description
Documentation
RemainAfterExit
DefaultDependencies
- 测试步骤
执行以下命令
```
cargo test --test docker_config_test -- docker_config_test_service_001 --exact --nocapture --ignored
```
- 测试结果

#### docker_config_test_service_002
- 测试内容
- 配置项
Type=foring
PIDFile
- 测试步骤
执行以下命令
```
cargo test --test docker_config_test -- docker_config_test_service_002 --exact --nocapture --ignored
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
1. 参考http://sysmaster.online/use/01-run%20sysmaster%20with%20vm/#sysmaster 搭建环境
2. 重启虚拟机，并以新启动项运行
3. 切换至tty1登录，通过ssh登录
- 测试结果
可以成功登录

#### service_001
- 测试内容
- 配置项
Description
Documentation
RemainAfterExit
DefaultDependencies
- 测试步骤
执行以下命令
```
cargo test --test vm_config_test -- service_001 --exact --nocapture --ignored
```
- 测试结果
