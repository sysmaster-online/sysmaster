# 测试框架及用例编写

## 概述

本文介绍了`tests`目录下集成测试用例的目录结构与执行方法，并以`docker_example_001`、`docker_config_test_service_001`等用例为案例介绍如何执行、编写集成测试用例。

## 用例目录结构

不同于单元测试用例，集成用例仅测试对外开放的接口。所有集成测试用例都放置在`tests`目录下。

```shell
[root@openEuler tests]# tree ./
./
├── test_frame.sh    # 测试框架
├── common    # 公共函数库目录
│   ├── docker_lib.sh    # 容器场景公共函数
│   ├── lib.sh    # test_frame.sh所需的变量和公共函数
│   ├── mod.rs    # run_script函数，测试用例入口函数
│   └── util_lib.sh    # 测试脚本（例如check.sh）公共函数
|   ...
├── docker_example.rs    # example测试套定义文件（容器场景）
├── docker_example    # example测试目录（容器场景）
│   └── docker_example_001    # example用例（容器场景）
│       ├── check.sh    # 测试主体，实际的检查脚本
│       └── docker_example_001.sh    # 测试用例入口脚本
|   ...
├── docker_config_test.rs    # 配置项测试套定义文件（容器场景）
├── docker_config_test    # 配置项测试目录（容器场景），其下文件都是指向config_test目录的软链接
│   ├── docker_config_test_service_001    # service_001用例（容器场景）
│   │   ├── check.sh -> ../../config_test/unit_config_test/service_001/check.sh
│   │   └── docker_config_test_service_001.sh -> ../../config_test/unit_config_test/service_001/service_001.sh
│   ├── ...
├── config_test    # 配置项测试的脚本归档目录（包括单元文件配置项、系统配置项等）
│   └── unit_config_test    # 单元文件配置项测试的脚本归档目录
│       ├── service_001    # service_001用例脚本归档目录
│       │   ├── service_001.sh    # 测试用例入口脚本
│       │   └── check.sh    # 测试主体，实际的检查脚本
│       ├── ...
|   ...
├── docker_reliable.rs    # 可靠性测试套定义文件（容器场景）
├── docker_reliable    # 可靠性测试目录（容器场景），其下文件都是指向reliable_test目录的软链接
│   └── docker_reliable_random_kill_001    # random_kill_001用例（容器场景）
│       ├── check.sh -> ../../reliable_test/random_kill_001/check.sh
│       └── docker_reliable_random_kill_001.sh -> ../../reliable_test/random_kill_001/random_kill_001.sh
├── reliable_test    # 可靠性测试的脚本归档目录
│   └── random_kill_001    # random_kill_001用例脚本归档目录
│       ├── check.sh    # 测试主体，实际的检查脚本
│       └── random_kill_001.sh    # 测试用例入口脚本
|   ...
├── presets
│   └── ...
└── test_units    # 测试所需文件的归档目录
    ├── basic.target
    ├── ...
    ├── tests    # 集成测试所需的单元文件归档目录
    │   ├── after.service
    │   ├── base.service
    │   ├── base.socket
    │   ├── ...
    ├── ...
```

在本项目中，`tests`目录下的每个rs文件代表一个用例集合，或者说一个测试套。rs文件命名需要遵循“场景\_模块”的规律，“模块”可扩展成“子模块”，例如：“场景\_模块\_a子模块\_b子模块\_...”。这样的命名风格不仅方便管理，也便于使用cargo test的原生功能进行用例筛选（具体见[用例执行]章节）。

```shell
├── docker_example.rs    # 场景：docker，模块后缀：example
├── docker_config_test.rs    # 场景：docker，模块后缀：config_test
├── vm_config_test.rs    # 场景：vm，模块后缀：config_test
├── docker_reliable.rs    # 场景：docker，模块后缀：reliable
```

`tests`目录下的每个rs文件须配套一个同名测试套目录，该目录下是用例的同名子目录，用于存放测试所需的shell脚本等文件。用例执行后生成的详细日志也会存放于此。用例命名须以其所在的rs文件名为前缀，后缀补充用例梗概信息，也可以在最后加上三位数字的编号用以区分。

```shell
├── docker_config_test.rs
├── docker_config_test    # rs文件同名测试套目录
│   ├── docker_config_test_condition_001    # 前缀：docker_config_test，用例梗概：condition配置测试，编号：001
│   ├── docker_config_test_condition_002    # 前缀：docker_config_test，用例梗概：condition配置测试，编号：002
│   ├── docker_config_test_dependency_001    # 前缀：docker_config_test，用例梗概：依赖类配置测试，编号：001
│   ├── docker_config_test_dependency_002    # 前缀：docker_config_test，用例梗概：依赖类配置测试，编号：002
│   ├── docker_config_test_env_001    # 前缀：docker_config_test，用例梗概：环境变量配置测试，编号：001
```

rs文件中以`#[test]`关键字定义测试用例。以`docker_config_test.rs`为例：

```rust
#[test]
#[ignore]
fn docker_config_test_dependency_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_002() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_002",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_env_001() {
    common::run_script("docker_config_test", "docker_config_test_env_001", "1");
}
```

你会注意到有一个`tests/common`目录，该目录用于存放一些公共函数库，不属于任何一个具体的用例。还有一个`test_units`目录，用于归档测试所需的单元文件，集成测试所需的单元文件都放置在`test_units/tests`目录下。

## 用例执行

推荐使用cargo工具进行自动化测试。在项目目录下执行以下命令就可以执行测试用例：

```shell
cargo clean
cargo build --all

# 只执行单元用例
cargo test
# 只执行集成用例
cargo test -- --ignored --test-threads=1
# 执行所有测试用例
cargo test -- --include-ignored --test-threads=1
```

#### 缓存清理

执行容器场景的用例，每次更新代码后必须清理环境中缓存的`sysmaster_base`容器镜像，再执行用例，否则被测对象无法更新。

```shell
[root@openEuler tests]# docker images
REPOSITORY                               TAG                 IMAGE ID            CREATED             SIZE
sysmaster_base-openeuler-22.03-lts-sp1   latest              0d044cccf14a        47 hours ago        461MB
openeuler-22.03-lts-sp1                  latest              a0213c9a6ecb        3 months ago        191MB
[root@openEuler tests]# docker rmi 0d044cccf14a
Untagged: sysmaster_base-openeuler-22.03-lts-sp1:latest
Deleted: sha256:0d044cccf14a33b03f19fb55cb7e5d8160ded6dde957ef657989b0d7b6069dbf
Deleted: sha256:334cc7ee8e6ff60f247fac390c441834f82a4dfa9c7ff912af6453d2c8e301cd
Deleted: sha256:43f31a4a00930231324af564494e6a8df8ea264a841e8cf3434ce5f6939ef3fe
Deleted: sha256:9c69b5de436cdf364f7804cef6c8e83d993763c1d2fac7d56a0a6057a0923541
Deleted: sha256:d42bdece43648694efcf0ce12d18696971188f3aab64910d8cb35b1696c84e4b
Deleted: sha256:15a448c8a65ea6827ab7348bde18f27b8e05afc3e634f9e670828cedb8f1a966
Deleted: sha256:937dfaf6419511399eec93ebac6b4d1a68fb68abf86214381d345be9fc1c11ae
Deleted: sha256:d7402eb5cba4f216aacd24fb189768a1b5658a47c6b9e9320787e497629c3622
[root@openEuler tests]# docker images
REPOSITORY                TAG                 IMAGE ID            CREATED             SIZE
openeuler-22.03-lts-sp1   latest              a0213c9a6ecb        3 months ago        191MB
```

#### ignore标记集成用例

社区ci门禁在容器中运行，所以无法直接在ci门禁中执行容器、虚拟机场景的用例。而ci门禁中会执行`cargo test --all`，如果不用`#[ignored]`关键字加以区分，会导致ci门禁中集成用例执行失败。

`#[ignore]`关键字具体什么含义呢？让我们先来看下官方usage信息：

```shell
Test Attributes:

    `#[test]`        - Indicates a function is a test to be run. This function
                       takes no arguments.
    `#[bench]`       - Indicates a function is a benchmark to be run. This
                       function takes one argument (test::Bencher).
    `#[should_panic]` - This function (also labeled with `#[test]`) will only pass if
                        the code causes a panic (an assertion failure or panic!)
                        A message may be provided, which the failure string must
                        contain: #[should_panic(expected = "foo")].
    `#[ignore]`       - When applied to a function which is already attributed as a
                        test, then the test runner will ignore these tests during
                        normal test runs. Running with --ignored or --include-ignored will run
                        these tests.
```

由此可知，`#[ignore]`标记的用例默认情况下不会执行，只有`cargo test`传参`--ignored` 或 `--include-ignored`才会执行；前者代表只执行带`#[ignore]`标记的用例，后者会执行带`#[ignore]`标记的用例和不带标记的用例。因此，本项目使用`#[ignore]`标记来跳过ci门禁中的集成用例。

#### 测试套执行

如果你只想执行单个用例，或者说单个测试套，可以使用cargo自带的用例名称筛选功能：

```shell
[root@openEuler tests]# cargo test --help
Execute all unit and integration tests and build examples of a local package

Usage: cargo test [OPTIONS] [TESTNAME] [-- [args]...]

Arguments:
  [TESTNAME]  If specified, only run tests containing this string in their names
  [args]...   Arguments for the test binary
```

从上述截取的usage信息中我们可以看到，`cargo test`后面的入参可以直接写用例名`[TESTNAME]`（只支持单个），或者用例名的子字符串。

以`docker_example_001`为例，我们可以直接尝试通过以下方式执行：

```shell
# 执行单个用例
catgo test docker_example_001

# 执行名称包含“docker_example”字符串的用例，即docker_example测试套
catgo test docker_example

# 执行名称包含“docker”字符串的用例，即所有容器场景的用例
catgo test docker
```

但上述传参方法只支持单个参数，如果你想执行前缀不同的多个测试套，可以参考如下命令：

```shell
cargo test --test docker_config_test --test docker_example
```

正如上述命令所示，`--test`选项可以指定测试对象，并且支持多次传参：

```shell
     --test [<NAME>]           Test only the specified test target
```

执行效果如下：

```shell
# Assume that docker_config_test_service_001 and docker_example_001 is the only testcase in their testsuits
[root@openEuler tests]# cargo test --test docker_config_test --test docker_example -- --ignored --test-threads=1
warning: /opt/sysmaster/Cargo.toml: `panic` setting is ignored for `bench` profile
warning: /opt/sysmaster/Cargo.toml: `panic` setting is ignored for `test` profile
   Compiling cmdproto v0.2.0 (/opt/sysmaster/libs/cmdproto)
warning: ExitStatus(unix_wait_status(0))
   Compiling sysmaster v0.2.2 (/opt/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 5.51s
     Running tests/docker_config_test.rs (/opt/sysmaster/target/debug/deps/docker_config_test-96302cf734a3a312)

running 1 test
test docker_config_test_service_001 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.42s

     Running tests/docker_example.rs (/opt/sysmaster/target/debug/deps/docker_example-b995116db87e6729)

running 1 test
test docker_example_001 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
```

你可能注意到上面的命令中，使用了`--test-threads`选项。默认情况下，cargo会多线程并发执行用例，这会有资源竞争的风险。因此必须在执行集成用例时使用`--test-threads=1`选项，限制后台线程数量，以达到串行执行的效果：

```shell
        --test-threads n_threads
                        Number of threads used for running tests in parallel
```

#### 日志输出

cargo test执行集成用例会打印用例路径、执行结果、耗时时长等信息。默认情况下，不打印用例成功的日志，只打印失败用例的详细日志（位置可能会有错乱），例如：

```shell
[root@openEuler tests]# cargo test --test docker_config_test --test docker_example -- --ignored --test-threads=1
warning: /opt/sysmaster/Cargo.toml: `panic` setting is ignored for `bench` profile
warning: /opt/sysmaster/Cargo.toml: `panic` setting is ignored for `test` profile
   Compiling cmdproto v0.2.0 (/opt/sysmaster/libs/cmdproto)
warning: ExitStatus(unix_wait_status(0))
   Compiling sysmaster v0.2.2 (/opt/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 5.41s
     Running tests/docker_config_test.rs (/opt/sysmaster/target/debug/deps/docker_config_test-96302cf734a3a312)

running 1 test
test docker_config_test_service_001 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 36.18s

     Running tests/docker_example.rs (/opt/sysmaster/target/debug/deps/docker_example-b995116db87e6729)

running 1 test
test docker_example_001 ... FAILED

failures:

---- docker_example_001 stdout ----
[ docker_example_001 ]: BUILD_PATH=/opt/sysmaster DOCKER_TEST=1 sh -x /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.sh &> /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.log
[ docker_example_001 ]: exit status: 1   Detail Log:
++ basename /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.sh
+ TEST_SCRIPT=docker_example_001.sh
++ dirname /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.sh
+ TEST_SCRIPT_PATH=/opt/sysmaster/tests/docker_example/docker_example_001
+ source /opt/sysmaster/tests/test_frame.sh
++ TEST_PATH=/opt/sysmaster/tests
++ source /opt/sysmaster/tests/common/lib.sh
+++ test -f /opt/sysmaster/target/release/sysmaster
+++ test -f /opt/sysmaster/target/debug/sysmaster
+++ MODE=debug
++ source /opt/sysmaster/tests/common/docker_lib.sh
+++ OS_VER=openEuler-22.03-LTS-SP1
++++ arch
+++ DOCKER_IMG_URL=https://mirrors.nju.edu.cn/openeuler/openEuler-22.03-LTS-SP1/docker_img/x86_64/
++++ arch
+++ DOCKER_TAR=openEuler-docker.x86_64.tar
+++ BASE_IMG=openeuler-22.03-lts-sp1
+++ SYSMST_BASE_IMG=sysmaster_base-openeuler-22.03-lts-sp1
++ source /opt/sysmaster/tests/common/util_lib.sh
+++ export EXPECT_FAIL=0
+++ EXPECT_FAIL=0
+++ export SYSMST_LIB_PATH=/usr/lib/sysmaster/system
+++ SYSMST_LIB_PATH=/usr/lib/sysmaster/system
+++ export SYSMST_RUN_PATH=/run/sysmaster/system
+++ SYSMST_RUN_PATH=/run/sysmaster/system
+++ export SYSMST_ETC_PATH=/etc/sysmaster/system
+++ SYSMST_ETC_PATH=/etc/sysmaster/system
+++ export SYSMST_LOG=/opt/sysmaster.log
+++ SYSMST_LOG=/opt/sysmaster.log
+++ export RELIAB_SWITCH_PATH=/run/sysmaster/reliability
+++ RELIAB_SWITCH_PATH=/run/sysmaster/reliability
+++ export RELIAB_SWITCH=switch.debug
+++ RELIAB_SWITCH=switch.debug
+++ export RELIAB_CLR=clear.debug
+++ RELIAB_CLR=clear.debug
+++ export init_pid=
+++ init_pid=
+++ export sysmaster_pid=
+++ sysmaster_pid=
+++ export 'cond_fail_log=Starting failed .* condition test failed'
+++ cond_fail_log='Starting failed .* condition test failed'
+++ export 'asst_fail_log=Starting failed .* assert test failed'
+++ asst_fail_log='Starting failed .* assert test failed'
+++ export yum_proxy=proxy=
+++ yum_proxy=proxy=
++ set +e
++ TMP_DIR=
+ set +e
+ runtest
+ local ret=1
+ log_info '===== cleanup before test ====='
++ date '+%F %T'
+ echo '[2023-04-13 20:19:29] [  INFO ] ===== cleanup before test ====='
[2023-04-13 20:19:29] [  INFO ] ===== cleanup before test =====
+ test_cleanup
+ '[' -n '' ']'
+ rm -rf /usr/bin/sctl /usr/lib/sysmaster
+ '[' 1 == 1 ']'
+ cleanup_docker
+ docker ps -a
+ grep -v 'CONTAINER ID'
+ docker images
+ grep -vEw 'IMAGE ID|openeuler-22.03-lts-sp1|sysmaster_base-openeuler-22.03-lts-sp1'
+ return 0
+ test_setup
+ setenforce 0
+ install_sysmaster
+ test -d /opt/sysmaster/target/install
+ return 0
+ '[' 1 == 1 ']'
+ setup_docker
++ mktemp -d /tmp/docker_example_001_XXXX
+ TMP_DIR=/tmp/docker_example_001_3Vx5
+ which docker
/usr/bin/docker
+ docker images
+ grep sysmaster_base-openeuler-22.03-lts-sp1
sysmaster_base-openeuler-22.03-lts-sp1   latest              399ae0ff19f9        28 hours ago        461MB
+ return 0
+ return 0
+ log_info '===== setup before test OK ====='
++ date '+%F %T'
+ echo '[2023-04-13 20:19:29] [  INFO ] ===== setup before test OK ====='
[2023-04-13 20:19:29] [  INFO ] ===== setup before test OK =====
++ type -t test_pre
+ '[' '' = function ']'
+ log_info '===== test_run begin ====='
++ date '+%F %T'
+ echo '[2023-04-13 20:19:29] [  INFO ] ===== test_run begin ====='
[2023-04-13 20:19:29] [  INFO ] ===== test_run begin =====
+ test_run
+ local ret
+ mkdir -p /tmp/docker_example_001_3Vx5/opt
++ realpath /opt/sysmaster/tests/docker_example/docker_example_001/check.sh
+ cp -arf /opt/sysmaster/tests/docker_example/docker_example_001/check.sh /tmp/docker_example_001_3Vx5/opt
+ chmod -R 777 /tmp/docker_example_001_3Vx5
+ docker run --privileged --rm -v /tmp/docker_example_001_3Vx5/opt:/opt sysmaster_base-openeuler-22.03-lts-sp1 sh -c 'sh -x /opt/check.sh &> /opt/check.log'
+ ret=1
+ cat /tmp/docker_example_001_3Vx5/opt/check.log
+ exit 1
+ return 1
+ log_info '===== test_run FAILED ====='
++ date '+%F %T'
+ echo '[2023-04-13 20:19:29] [  INFO ] ===== test_run FAILED ====='
[2023-04-13 20:19:29] [  INFO ] ===== test_run FAILED =====
+ log_info '===== cleanup after test ====='
++ date '+%F %T'
+ echo '[2023-04-13 20:19:29] [  INFO ] ===== cleanup after test ====='
[2023-04-13 20:19:29] [  INFO ] ===== cleanup after test =====
+ test_cleanup
+ '[' -n /tmp/docker_example_001_3Vx5 ']'
+ rm -rf /tmp/docker_example_001_3Vx5
+ rm -rf /usr/bin/sctl /usr/lib/sysmaster
+ '[' 1 == 1 ']'
+ cleanup_docker
+ docker ps -a
+ grep -v 'CONTAINER ID'
+ docker images
+ grep -vEw 'IMAGE ID|openeuler-22.03-lts-sp1|sysmaster_base-openeuler-22.03-lts-sp1'
+ return 0
+ exit 1
thread 'docker_example_001' panicked at 'assertion failed: status.success()', tests/common/mod.rs:40:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    docker_example_001

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s

error: test failed, to rerun pass `--test docker_example`
```

`---- docker_example_001 stdout ----`下的信息就是`docker_example_001`的详细执行日志，它明确告诉我们具体失败的位置。如果你想查看成功用例的详细日志信息，可以尝试`--show-output`选项：

```shell
        --show-output   Show captured stdout of successful tests
```

在调试定位的过程中，cargo直接打印在屏幕上的信息很容易丢失。我们可以根据提示找到详细日志的归档位置，即用例入口脚本的同级目录。

> 注意！每次执行用例都会覆盖上一次的执行日志。

```shell
[ docker_example_001 ]: BUILD_PATH=/opt/sysmaster DOCKER_TEST=1 sh -x /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.sh &> /opt/sysmaster/tests/docker_example/docker_example_001/docker_example_001.log
```

根据示例中的失败日志，我们可以发现，测试用例实际上是shell脚本的rust封装。用例通过`#[test]`标记在rs文件中注册函数。cargo通过这些函数执行对应的测试入口脚本。而入口脚本也只是一层封装，实际的测试主体是另一个检查脚本。

```
docker_example.rs  ------>  测试套文件
      |
      |
   fn docker_example_001  ------>  #[test]注册函数，即注册用例
            |
            |
      sh -x docker_example_001.sh  ------>  测试入口脚本，与用例同名，对应日志：docker_example_001.log
                  |
                  |
            sh -x check.sh  ------>  测试主体检查脚本
```

你可能会疑问，为什么主体检查脚本外还要再封装一层入口脚本？这是因为sysmaster要支持容器、虚拟机等场景。不同场景的环境准备工作不同，拉起检查脚本的方式也不同，但实际检查的内容却几乎相同。因此，设置入口脚本用于差异化环境准备，检查脚本用于执行共性的测试步骤。

## 用例编写

了解如何执行用例后 ，我们来尝试自己编写一个集成测试用例。首先，回顾一下上文提到的要点：

- rs文件即测试套文件，用于注册测试函数，一个测试函数就是一个测试用例。
- 测试套命名遵循“场景\_模块”风格，测试用例以测试套名称为前缀。
- 测试入口脚本用于环境准备，测试检查脚本才是真正的主体，各脚本需严格分级归档。

牢记以上三点，让我们从rs文件开始写用例吧。

#### rs文件

以容器场景的`docker_example.rs`为例，首先`mod common`导入`common::run_script`函数。`fn docker_example_001`就是测试函数，也就是cargo识别到的真正用例。

```rust
mod common;

#[test]
#[ignore]
fn docker_example_001() {
    common::run_script("docker_example", "docker_example_001", "1");
}
```

`docker_example_001`函数中只有一行调用，即`run_script`，该函数在`common/mod.rs`中定义。`run_script`一共有3个字符串类型入参，分别是：

- 测试套名称suit
- 测试用例名称name
- DOCKER_TEST的值

前两个入参用于拼接测试入口脚本的路径，即`{suit}/{name}/{name}.sh`。第三个参数等于1表示是容器场景，测试入口脚本会根据这个变量的值部署环境。

#### 测试入口脚本

以`docker_example/docker_example_001/docker_example_001.sh`为例，入口脚本首先需要定义2个全局变量`TEST_SCRIPT`、`TEST_SCRIPT_PATH`，这两者是必须的，每个脚本的开头都必须定义，用于获取source路径，可以直接拷贝样例中的定义。

```shell
TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(dirname "$0")"
```

接着需要`source test_frame.sh`，该文件定义了测试框架的几个关键函数，[框架函数]小节再展开介绍。然后，就需要测试人员根据测试场景自行编写`test_run`函数。

`docker_example_001`用例是容器场景用例，因此`test_run`函数需要进行一些环境准备，例如：创建临时目录，将检查脚本`check.sh`拷贝至临时目录。然后，通过`docker run`命令挂载临时目录并执行`check.sh`。最后，记录返回值、打印检查脚本的执行日志。这一系列操作是在容器场景下是通用的，可以直接移植。

```shell
#!/bin/bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(dirname "$0")"

source "${BUILD_PATH}"/tests/test_frame.sh
set +e

function test_run() {
    local ret
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "$(realpath "${TEST_SCRIPT_PATH}"/check.sh)" "${TMP_DIR}"/opt
    chmod -R 777 "${TMP_DIR}"
    docker run --privileged --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest
```

如果是虚拟机场景的用例呢？可以参考`docker_config_test_service_001`用例的入口脚本：

```shell
[root@openEuler tests]# cat docker_config_test/docker_config_test_service_001/docker_config_test_service_001.sh
#!/bin/bash
# Description: test for Description/Documentation/RemainAfterExit/DefaultDependencies

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(dirname "$0")"

source "${BUILD_PATH}"/tests/test_frame.sh
set +e

function test_pre() {
    pushd "${TEST_SCRIPT_PATH}"
    rm -rf tmp_units
    mkdir tmp_units
    cp -arf "${TEST_PATH}"/test_units/{shutdown.target,sysinit.target} tmp_units
    cp -arf "${TEST_PATH}"/test_units/tests/base.service tmp_units
    popd
}

function test_run() {
    local ret

    pushd "${TEST_SCRIPT_PATH}"
    if [ "${DOCKER_TEST}" == '1' ]; then
        mkdir -p "${TMP_DIR}"/opt
        cp -arf "$(realpath check.sh)" "${TMP_DIR}"/opt
        cp -arf "${TEST_PATH}"/common/util_lib.sh tmp_units "${TMP_DIR}"/opt
        chmod -R 777 "${TMP_DIR}"
        docker run --privileged --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
        ret=$?
        cat "${TMP_DIR}"/opt/check.log
        cat "${TMP_DIR}"/opt/sysmaster.log
    else
        cp -arf "${TEST_PATH}"/common/util_lib.sh ./
        sh -x check.sh &> check.log
        ret=$?
        cat check.log
        cat sysmaster.log
    fi

    rm -rf tmp_units check.log
    popd
    return "${ret}"
}

runtest
```

上述入口脚本的`test_run`含有一个if分支，分支根据`DOCKER_TEST`变量的值判断场景，并根据不同的场景做一些差异性的环境准备，再以不同的方式拉起`check.sh`。而一些共性的环境准备工作，可以在`test_pre`中执行，例如拷贝测试所需的单元文件等。

另外，有很多用例是多场景通用的，这些用例的入口脚本实际上是一个软链接，例如`docker_config_test_service_001.sh`：

```shell
[root@openEuler tests]# ll docker_config_test/docker_config_test_service_001/docker_config_test_service_001.sh
lrwxrwxrwx. 1 root root 61 Apr 11 17:28 docker_config_test/docker_config_test_service_001/docker_config_test_service_001.sh -> ../../config_test/unit_config_test/service_001/service_001.sh
```

主体检查脚本同样也可以设置软链接，例如：

```shell
[root@openEuler tests]# ls -l docker_config_test/docker_config_test_service_001/check.sh
lrwxrwxrwx. 1 root root 55 Apr 14 14:26 docker_config_test/docker_config_test_service_001/check.sh -> ../../config_test/unit_config_test/service_001/check.sh
```

在虚拟机场景测试相应的配置时，只需在`vm_config_test_service_001`目录下新建`vm_config_test_service_001.sh`、`check.sh`软链接，分别指向`service_001`目录下的`service_001.sh`、`check.sh`文件。并在注册对应的测试函数时，将第3个入参设置为0，由此便可尽可能地减少冗余代码：

```rust
#[test]
#[ignore]
fn vm_config_test_service_001() {
    common::run_script("vm_config_test", "vm_config_test_service_001", "0");
}
```

#### 框架函数

接下来，就让我们了解一下`test_frame.sh`是如何定义`runtest`函数的。`test_frame.sh`位于`tests`目录下。`runtest`函数其实非常简单，一共分四个阶段：

- test\_cleanup：测试前环境清理，清理失败则用例失败退出。
- test\_setup：环境部署，包括构建基础容器镜像等；部署失败则用例失败退出。
- test\_run：需要测试人员在入口脚本中自行定义；执行失败则用例失败，但不会立即退出。
- test\_cleanup：测试后的环境清理，无论test\_run成功与否都会执行，清理失败也不影响用例结果。

若环境中没有执行过容器场景的用例，`test_setup`会从openEuler官网下载标准容器镜像并导入，再将sysmaster编译出的二进制和lib库文件拷贝至标准容器镜像，由此构建基础镜像，用作后续测试。若环境中已经执行过容器场景的用例，`test_setup`检测到可用的基础镜像，就不会再重复构建。

> 注意！更新代码后，必须手动删除缓存的基础镜像，确保二进制更新。

`test_setup`还会在`/tmp`目录下创建一个以用例名命名的临时目录，用于存放一些临时文件。

`test_cleanup`会在测试前后清理环境中的残留容器和镜像，但不会删除归档的基础镜像和标准镜像。值得注意的是，测试后的`test_cleanup`会删除`test_setup`创建的`/tmp`临时目录，测试前的`test_cleanup`不会。

#### 检查脚本

检查脚本是测试用例的主体，本项目中大部分检查脚本都以`check.sh`命名。其中的可以设置多个检查函数，以`docker_config_test_service_001`为例：

```shell
[root@openEuler tests]# cat docker_config_test/docker_config_test_service_001/check.sh
#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Description/Documentation/RemainAfterExit
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1

    # RemainAfterExit=false
    sed -i 's/^Description=.*/Description="this is a test"/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a Documentation="this is doc"' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ a RemainAfterExit=false' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    check_status base inactive || return 1
    # check Description/Documentation
    sctl status base | grep "base.service - this is a test" && sctl status base | grep "Docs: this is doc"
    expect_eq $? 0 || sctl status base
    # clean
    kill_sysmaster

    # RemainAfterExit=true
    sed -i '/RemainAfterExit/ s/false/true/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    sctl status base | grep active | grep 'running'
    expect_eq $? 0 || sctl status base
    main_pid="$(get_pids base)"
    sleep 2
    check_status base active || return 1
    sctl status base | grep active | grep 'exited'
    expect_eq $? 0 || sctl status base
    ps -elf | grep -v grep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf

    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster
}

# usage: test RemainAfterExit with oneshot service
function test02() {
    log_info "===== test02 ====="
    ...
}

# usage: test DefaultDependencies
function test03() {
    local key_log='add default dependencies for target.*'

    log_info "===== test03 ====="
    ...
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
```

上述检查脚本中一共有3个检查函数：`test01`、`test02`、`test03`。每个检查函数异常返回都会导致脚本直接失败退出，任意一个断言失败也会导致脚本最终非0退出：

```shell
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
```

以`test01`为例，首先将临时目录下的单元文件`base.service`拷贝至sysmaster对应目录，并修改其中的配置。然后拉起sysmaster，通过`sctl`等命令进行测试。`expect_eq`等断言失败不会直接导致函数退出，而是会将`EXPECT_FAIL`变量置1，检查脚本最后会根据该变量的值判断用例成功与否。

`check_status`、`log_info`等公共函数，以及`expect_xxx`系列断言函数，都是在`common/util_lib.sh`中定义的。编写测试用例前可以先熟悉一下现有的公共函数库。



以上重点介绍了容器场景的测试框架和用例编写，随着测试场景的丰富，后续会跟进补充。

## FAQ



## 参考资料

1. [Rust程序设计语言 简体中文版 —— 编写自动化测试](http://kaisery.github.io/trpl-zh-cn/ch11-00-testing.html)
2. [Rust语言圣经 —— 自动化测试](http://course.rs/test/intro.html)
