# sysMaster 基础功能测试

**测试内容：**

1. 基本功能：支持x86_64、aarch64；
2. 可靠性：服务查询和操作类脚本与reload并发，sysmaster无崩溃和内存泄漏；
3. 性能：nginx重启1000次，sysmaster相对于systemd占用CPU时间减少30%。

## 1 搭建测试环境

### 1.1 安装构建工具

下载仓库代码及安装工具链, 生成的二进制程序位于`target`下.

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

### 1.2 部署容器测试环境

> 注: 可通过`cargo test`执行集成测试用例时会自动化部署，无需手动执行。
>

- 归档二进制，归档后的二进制位于`target/install`目录下，用于构建容器镜像.

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

wget -P "${TMP_DIR}"
"${DOCKER_IMG_URL}/${DOCKER_TAR}".xz

xz -d
"${TMP_DIR}"/"${DOCKER_TAR}".xz

docker load --input
"${TMP_DIR}"/"${DOCKER_TAR}"

docker images
```

- 构建测试镜像


```bash
pushd "${TMP_DIR}"

cat << EOF > Dockerfile

FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}

COPY install/usr/bin/sctl /usr/bin/

COPY install/usr/bin/init /usr/bin/

RUN mkdir /usr/lib/sysmaster

COPY install/usr/lib/sysmaster
/usr/lib/sysmaster/

EOF

cat Dockerfile

docker build -t "${SYSMST_BASE_IMG}:latest"
.

popd
```

# 2 基本功能测试方案

## 2.1 测试内容

基于容器场景，在x86_64、aarch64两种架构上测试单元的启停和查询功能。

## 2.2 测试平台

- x86_64


| **工具/** **硬件** | **参数**                                  |
| ------------------ | ----------------------------------------- |
| **设备类型**       | 2288H V5                                  |
| **CPU**            | Intel(R) Xeon(R) Gold 5218R CPU @ 2.10GHz |
| **内存**           | 8*16G                                     |
| **硬盘**           | SAS3408                                   |
| **虚拟机操作系统** | openEuler-22.03-LTS-SP1                   |
| **docker版本**     | 18.09.0                                   |
| **gcc版本**        | 10.3.1                                    |
| **musl-gcc版本**   | 1.2.3                                     |

- aarch64


| **工具/** **硬件** | **参数**                    |
| ------------------ | --------------------------- |
| **设备类型**       | Taishan 200                 |
| **CPU**            | Kunpeng920-6426 @ 2.6GHz *2 |
| **内存**           | 251G                        |
| **硬盘**           | 1.5T                        |
| **虚拟机操作系统** | openEuler-22.03-LTS-SP1     |
| **docker版本**     | 18.09.0                     |
| **gcc版本**        | 10.3.1                      |
| **musl-gcc版本**   | 1.2.3                       |

## 2.3 测试步骤

执行docker_config_test_service_002用例

``` bash
cargo test docker_config_test_service_002 --
--ignored --test-threads=1
```

> 备注：该用例覆盖了forking类型单元文件的启动、停止以及状态查询。
>

### 2.4 测试结果



# 3 可靠性及性能测试方案

## 3.1 测试内容

1. 编写dbus、udev、journal、login、systemctl、cgroup等子组件的查询和操作类脚本，后台并发持续运行，且dbus、重构1号进程每天reload一次，一周内整个系统运行正常，无复位重启、内存泄露和内存越界等安全问题。
2. 在systemd-journald、nginx、mysql、redis等常用任务重启500次条件下，重构的1号进程相对于开源Systemd，占用CPU时间减少20%。

## 3.2 测试平台

| **工具/** **硬件** | **参数**                                  |
| ------------------ | ----------------------------------------- |
| **设备类型**       | 2288H V5                                  |
| **CPU**            | Intel(R) Xeon(R) Gold 5218R CPU @ 2.10GHz |
| **内存**           | 8*16G                                     |
| **硬盘**           | SAS3408                                   |
| **虚拟机操作系统** | openEuler-22.03-LTS-SP1                   |
| **docker版本**     | 18.09.0                                   |
| **system版本**     | 249-43                                    |
| **nginx版本**      | 1.21.5                                    |

## 3.3 测试步骤

执行docker_reliable_reload_001用例

```bash
cargo test docker_reliable_reload_001 --
--ignored --test-threads=1
```

> 备注：用例会后台记录sysmaster资源占用情况，需人工观察资源走势。
>

执行docker_perf_001用例

```bash
cargo test docker_perf_001 -- --ignored
--test-threads=1
```

> 备注：该用例基于nginx服务，测试服务间隔0.1s反复重启1000次，记录sysmaster进程的用户态和内核态耗时。
>

systemd基线数据收集

```bash
yum install -y nginx

sed -i '/Description=/a StartLimitBurst=0'
/usr/lib/systemd/system/nginx.service

systemctl daemon-reload

cat /proc/1/stat | awk '{print $14,$15}' >>
/tmp/result

for ((i=0; i<1000; ++i)); do
    systemctl restart nginx
    sleep 0.1
done

cat /proc/1/stat | awk '{print $14,$15}' >>
/tmp/result
```

> 备注：/tmp/result文件中的两行数据分别代表测试前、后1号进程的耗时，差值便是重启nginx服务1000次的耗时，第一列是用户态，第二列是内核态。
>

## 3.4 测试结果

- docker_reliable_reload_001用例连续运行 **X** **天** ，sysmaster进程占用RSS和fd无明显增长趋势，趋于稳定：

- nginx服务重启500次，sysmaster对比systemd CPU耗时降级 **20%** ：
- nginx服务重启1000次，sysmaster对比systemd性能数据（单位：jiffies）

|             | sysmaster | systemd | 性能提升 |
| ----------- | --------- | ------- | -------- |
| 用户态utime |           |         |          |
| 内核态stime |           |         |          |
| 总和        |           |         |          |

# 参考资料

如何添加测试用例, 请参考 [测试框架及用例编写](../design/01-integration_test.md)
