# 集成测试用例



## 概述

本文介绍了```tests```目录下集成测试用例的结构与执行方法，并以```docker_example_001```为例介绍如何编写一个集成测试用例。



## 用例结构

不同于单元测试用例，集成用例仅测试对外开放的接口。所有集成测试用例都放置在```tests```目录下。

```
[root@localhost tests]# tree
.
├── common
│   ├── docker_lib.sh
│   ├── lib.sh
│   ├── log.sh
│   ├── mod.rs
│   └── test_frame_docker.sh
├── docker_example_001
│   ├── check.sh
│   ├── docker_example_001.log
│   └── docker_example_001.sh
└── docker_example_001.rs
```

在本项目中，```tests```目录下的一个rs文件就代表一个用例。用例命名需要遵循“场景\_模块\_编号”的规律，“模块”可扩展成“子模块”，例如：“场景\_模块\_a子模块\_b子模块\_xxx\_编号”。当前的集成用例主要是测试容器场景，因此，所有rs文件的命令都以“docker”开头。这样的命名风格不仅方便管理，也便于使用cargo test的原生功能进行用例筛选（具体见用例执行章节）。

```tests```目录下的每个rs文件还会有一个配套的同名目录，用于存放测试所需的shell脚本、unit配置等文件。用例执行后生成的详细日志也会存放在这个同名目录下。

你会注意到有一个```tests/common```目录，该目录用于存放一些公共函数库，不属于任何一个具体的用例。

另外，为了风格统一和执行便利，测试用例必须直接放置在```tests```目录下，不要创建子目录放置用例入口rs文件。



## 用例执行

推荐使用cargo工具进行自动化测试。在项目目录下执行```cargo test```就可以执行所有测试用例，包括单元用例和集成用例。

如果你只想执行集成用例，或者说部分集成用例，可以使用cargo自带的用例名称字符串筛选功能：

```
[root@localhost tests]# cargo test --help
cargo-test
Execute all unit and integration tests and build examples of a local package

USAGE:
    cargo test [OPTIONS] [TESTNAME] [-- <args>...]

ARGS:
    <TESTNAME>    If specified, only run tests containing this string in their names
    <args>...     Arguments for the test binary
```

从上述截取的usage信息中我们可以看到，```cargo test```后面的入参可以直接写用例名[TESTNAME]（只支持单个），或者用例名的子字符串。

以```docker_example_001```为例，我们可以直接执行```catgo test docker_example_001```，意思是只执行```docker_example_001```这个用例。或者，我们也可以执行```cargo test docker```，cargo会自动寻找所有名称中带有“docker”字符串的用例并执行。这里就充分体现了“场景\_模块\_编号”命名风格的优势，相同场景/模块的用例具有相同的名称前缀，在自动化执行时可以自定义深度进行用例筛选。

在上述基础上，如果你想跳过个别用例，推荐你使用ignore关键字。只需编辑你想跳过的用例rs文件，在```[test]```关键字下追加```[ignore]```关键字，这个用例就会被忽略。反之，你如果只想执行这些被忽略的用例，只需使用```--ignored```参数，cargo会仅执行被ignore关键字标注过的用例：

```
cargo test -- --ignored
```

默认情况下，cargo会多线程并发执行用例，这会有资源竞争的风险。建议在执行集成用例时使用```--test-threads```参数，限制后台线程数量以达到串行执行的效果（在串行执行模式下，当有一个用例执行失败，后续用例就不会再执行）：

```
cargo test -- --test-threads=1
```

现在，我们来看一下用例执行的输出（注意！需要先删除docker_example_001.rs文件中的[ignore]标记！）。

```
[root@localhost tests]# cargo test --test docker_example_001 -- --test-threads=1
   Compiling sysmaster v0.1.0 (/root/sysmaster/sysmaster)
   Compiling sysmaster v0.2.2 (/root/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 6.54s
     Running tests/docker_example_001.rs (/root/sysmaster/target/debug/deps/docker_example_001-21f1c699b9d8d520)

running 1 test
test docker_example_001 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
```

如上所示，当我们使用```--test```参数指定执行某个用例时，cargo会打印用例路径、执行结果、耗时时长等信息，但不会打印用例日志。默认情况下，cargo只会打印失败用例的详细日志，例如：

```
[root@localhost tests]# cargo test --test docker_example_002 -- --test-threads=1
   Compiling sysmaster v0.1.0 (/root/sysmaster/sysmaster)
   Compiling sysmaster v0.2.2 (/root/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 5.86s
     Running tests/docker_example_002.rs (/root/sysmaster/target/debug/deps/docker_example_002-c2ed27402aa5f112)

running 1 test
test docker_example_002 ... FAILED

failures:

---- docker_example_002 stdout ----
[ docker_example_002 ]: sh -x /root/sysmaster/tests/docker_example_002/docker_example_002.sh &> /root/sysmaster/tests/docker_example_002/docker_example_002.log
[ docker_example_002 ]: exit status: 2
thread 'main' panicked at 'assertion failed: status.success()', tests/common/mod.rs:16:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    docker_example_002

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass '--test docker_example_002'
```

```---- docker_example_002 stdout ----```下的信息就是```docker_example_002```的执行日志，它明确告诉我们具体失败的位置，以及用例rs文件执行过程中的显示打印。如果你想查看成功用例的详细日志信息，可以尝试```--show-output ```参数：

```
[root@localhost tests]# cargo test --test docker_example_001 -- --test-threads=1 --show-output
   Compiling sysmaster v0.1.0 (/root/sysmaster/sysmaster)
   Compiling sysmaster v0.2.2 (/root/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 5.60s
     Running tests/docker_example_001.rs (/root/sysmaster/target/debug/deps/docker_example_001-21f1c699b9d8d520)

running 1 test
test docker_example_001 ... ok

successes:

---- docker_example_001 stdout ----
[ docker_example_001 ]: sh -x /root/sysmaster/tests/docker_example_001/docker_example_001.sh &> /root/sysmaster/tests/docker_example_001/docker_example_001.log
[ docker_example_001 ]: exit status: 0


successes:
    docker_example_001

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s
```

现在我们可以看到```---- docker_example_001 stdout ----```，但是不会有报错信息。

上面的示例中我们用到了```--test```参数，但是必须指定完整的用例名，假设我们传入用例名的子字符串，会出现如下报错：

```
[root@localhost tests]# cargo test --test docker_example -- --test-threads=1
error: no test target named `docker_example`
```

如果想用字符串进行用例筛选，可以这样执行：

```
[root@localhost tests]# cargo test example -- --test-threads=1
   Compiling sysmaster v0.1.0 (/root/sysmaster/sysmaster)
   Compiling sysmaster v0.2.2 (/root/sysmaster)
    Finished test [unoptimized + debuginfo] target(s) in 8.78s
     Running unittests src/fstab/main.rs (/root/sysmaster/target/debug/deps/fstab-8c2a561dde232947)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

     Running unittests src/init/main.rs (/root/sysmaster/target/debug/deps/init-cca98375302574a2)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/pctrl/main.rs (/root/sysmaster/target/debug/deps/pctrl-efe504cbad79e269)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/random_seed/main.rs (/root/sysmaster/target/debug/deps/random_seed-d86160bb56b29215)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s

     Running unittests src/rc-local-generator/main.rs (/root/sysmaster/target/debug/deps/rc_local_generator-ef053c1800388d72)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running unittests src/sysmaster/main.rs (/root/sysmaster/target/debug/deps/sysmaster-c0d6d8d4f166b7b0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/sysmonitor/main.rs (/root/sysmaster/target/debug/deps/sysmonitor-b77a0affced65af5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/docker_another_001.rs (/root/sysmaster/target/debug/deps/docker_another_001-01f69a2a430b6732)

running 0 tests

     Running tests/docker_example_001.rs (/root/sysmaster/target/debug/deps/docker_example_001-21f1c699b9d8d520)

running 1 test
test docker_example_001 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s

     Running tests/docker_example_002.rs (/root/sysmaster/target/debug/deps/docker_example_002-c2ed27402aa5f112)

running 1 test
test docker_example_002 ... FAILED

failures:

---- docker_example_002 stdout ----
[ docker_example_002 ]: sh -x /root/sysmaster/tests/docker_example_002/docker_example_002.sh &> /root/sysmaster/tests/docker_example_002/docker_example_002.log
[ docker_example_002 ]: exit status: 2
thread 'main' panicked at 'assertion failed: status.success()', tests/common/mod.rs:16:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    docker_example_002

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass '--test docker_example_002'
```

如上所示，你会看到冗长的输出。这是因为cargo会全局搜索包含”example“字符串的用例名，那些不符合的用例虽然有打印信息，但并不会真正执行。你可以看到，除了```docker_example_001```和```docker_example_002```之外，其他所有用例都是```filtered out```状态，也都有```running 0 tests```打印。

细心的你可能会疑惑，为什么有的rs文件会包含多个用例？这常见于```unittests```，也就是单元测试用例。实际上，每个被```[test]```关键字标注的函数都可以成为一个用例。当一个rs文件包含多个标注过的测试函数时，cargo就会识别到多个用例。理论上，```tests```目录下的rs文件也可以设置多个测试函数，但本项目的测试框架规定一个集成用例rs文件只能提供一个测试函数，且函数名必须与rs文件名相同。其实，```unittests```和```tests```还有很多微妙的区别，比如后者不需要配置```[cfg(test)]```关键字等，篇幅限制，本文档不做展开。

很明显，cargo直接打印的信息并不能支撑我们深入定位失败根因。这是因为我们的集成用例rs文件其实只是一个封装，实际调用的是同名目录下的同名shell脚本，shell脚本的详细执行日志被重定向到同级的同名日志文件中。cargo的详细日志只会打印调用shell脚本的语句和shell脚本的返回值：

```
---- docker_example_001 stdout ----
[ docker_example_001 ]: sh -x /root/sysmaster/tests/docker_example_001/docker_example_001.sh &> /root/sysmaster/tests/docker_example_001/docker_example_001.log
[ docker_example_001 ]: exit status: 0
```

如上所示，我们可以前往```/root/sysmaster/tests/docker_example_001/docker_example_001.log```查看用例失败的具体原因。



## 用例编写

了解如何执行用例后 ，我们来尝试自己编写一个集成测试用例。首先，回顾一下上文提到的要点：

- 集成测试用例rs文件直接放置在`tests`目录下。
- 一个rs文件只包含一个测试函数，函数与文件同名。
- rs文件的同级同名目录中放置同名shell入口脚本。

牢记以上三点，让我们从rs文件开始写用例吧。以容器场景的`docker_example_001.rs`为例，首先`mod common`导入`common::run_script`函数。`fn docker_example_001`就是测试函数，也就是cargo识别到的真正用例。测试函数只做一件事，就是通过传参执行对应名称的shell脚本，一般只需要将测试函数名直接作为参数进行传递。

```
[root@localhost tests]# cat docker_example_001.rs
mod common;

#[test]
fn docker_example_001() {
    common::run_script("docker_example_001");
}
```

`run_script`在`common/mod.rs`中定义，这个函数的主要作用就是执行指定名称的shell脚本，并判断返回值。

接下来让我们开始写shell脚本。以`docker_example_001/docker_example_001.sh`为例，shell脚本首先需要定义三个全局变量`TEST_SCRIPT`、`TEST_SCRIPT_PATH`、`TEST_PATH`，这三者是必须的，每个脚本的开头都必须定义。接着需要`source test_frame_docker.sh`，该文件定义了测试框架的几个关键函数，后续再展开介绍。然后，就需要测试人员根据测试目的自行编写`test_run`函数，即测试的主体函数。最后，调用`runtest`函数。由此可见，测试脚本的真正入口函数是`runtest`函数。
```
[root@localhost tests]# cat docker_example_001/docker_example_001.sh
#!/bin/bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(realpath "$0")"
TEST_SCRIPT_PATH="${TEST_SCRIPT_PATH%/${TEST_SCRIPT}}"
TEST_PATH="$(dirname "${TEST_SCRIPT_PATH}")"

set +e
source "${TEST_PATH}"/common/test_frame_docker.sh

function test_run() {
    local ret
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "${TEST_SCRIPT_PATH}"/check.sh "${TMP_DIR}"/opt
    chmod 777 "${TMP_DIR}"/opt/check.sh
    docker run --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest
```

接下来，就让我们了解一下`test_frame_docker.sh`是如何定义`runtest`函数的。`test_frame_docker.sh`位于`common`目录下。`runtest`函数其实非常简单，一共分四个阶段：

- test_cleanup：测试前环境清理，清理失败则用例失败退出。
- test_setup：环境部署，包括构建基础容器镜像等；部署失败则用例失败退出。
- test_run：测试主体，需要测试人员在shell入口脚本中自行定义；执行失败则用例失败，但不会立即
退出。
- test_cleanup：测试后环境清理，无论用例主体成功与否都会执行，清理失败也不影响用例结果。

```
function runtest() {
    local ret=1

    if ! test_cleanup; then
        log_error "===== cleanup before test failed, exit! ====="
        exit 1
    fi

    if ! test_setup; then
        log_error "===== setup before test failed, exit! ====="
        exit 1
    fi

    if test_run; then
        log_info "===== test_run OK ====="
        ret=0
    else
        log_info "===== test_run FAILED ====="
    fi
    test_cleanup

    exit "${ret}"
}
```

若环境中没有执行过容器场景的用例，`test_setup`会从openEuler官网下载标准容器镜像并导入，再将sysmaster编译出的二进制和lib库文件拷贝至标准容器镜像，由此构建基础镜像，用作后续测试。若环境中已经执行过容器场景的用例，`test_setup`检测到可用的基础镜像，就不会再重复构建。`test_setup`还会在`/tmp`目录下创建一个以用例名命名的临时目录，用于存放一些临时文件。

`test_cleanup`会在测试前后清理环境中的残留容器和镜像，但不会删除归档的基础镜像和标准镜像。值得注意的是，测试后的`test_cleanup`会删除`test_setup`创建的`/tmp`临时目录，测试前的`test_cleanup`不会。

自定义的`test_run`函数是测试的重点。建议在`test_run`函数中，以挂载`/tmp`临时目录的形式，将子测试脚本传递到容器内部，便于容器内部调用。例如，`docker_example_001.sh`通过`docker run`的
`-v`参数将`check.sh`子脚本传递至容器内部，并在容器中执行。执行结果也记录在临时挂载目录下的`check.log`中。`check.sh`子脚本检查了基础镜像中是否存在sysmaster组件的二进制文件，若不存在则失败退出。

```
[root@localhost tests]# cat docker_example_001/check.sh
ls -l /usr/lib/sysmaster || exit 1
ls -l /usr/lib/sysmaster/plugin || exit 1
ls -l /usr/bin/pctrl || exit 1
```

详细日志如下：

```
+ test_run
+ local ret
+ mkdir -p /tmp/docker_example_001_mTTo/opt
+ cp -arf /root/sysmaster/tests/docker_example_001/check.sh /tmp/docker_example_001_mTTo/opt
+ chmod 777 /tmp/docker_example_001_mTTo/opt/check.sh
+ docker run --rm -v /tmp/docker_example_001_mTTo/opt:/opt sysmaster_base-openeuler-22.09 sh -c 'sh -x /opt/check.sh &> /opt/check.log'
+ ret=0
+ cat /tmp/docker_example_001_mTTo/opt/check.log
+ ls -l /usr/lib/sysmaster
total 28344
-rwxr-xr-x. 1 root root 3994760 Oct 20 20:30 fstab
-rwxr-xr-x. 1 root root 4003008 Oct 20 20:30 init
drwxr-x---. 1 root root    4096 Oct 21 11:31 plugin
-rwxr-xr-x. 1 root root 4354720 Oct 20 20:30 random_seed
-rwxr-xr-x. 1 root root 4294784 Oct 20 20:30 rc-local-generator
-rwxr-xr-x. 1 root root 7925360 Oct 20 20:30 sysmaster
-rwxr-xr-x. 1 root root 4432992 Oct 20 20:30 sysmonitor
+ ls -l /usr/lib/sysmaster/plugin
total 6884
-rw-r--r--. 1 root root  612640 Oct 21 11:31 libmount.so
-rw-r--r--. 1 root root 2464064 Oct 21 11:31 libservice.so
-rw-r--r--. 1 root root 3303776 Oct 21 11:31 libsocket.so
-rw-r--r--. 1 root root  657696 Oct 21 11:31 libtarget.so
-rw-r--r--. 1 root root      68 Oct 18 11:35 plugin.conf
+ ls -l /usr/bin/pctrl
-rwxr-xr-x. 1 root root 4730576 Oct 20 20:30 /usr/bin/pctrl
+ return 0
+ log_info '===== test_run OK ====='
++ date '+%F %T'
+ echo '[2022-10-21 17:34:41] [  INFO ] ===== test_run OK ====='
[2022-10-21 17:34:41] [  INFO ] ===== test_run OK =====
+ ret=0
```

如果你想测试某个service配置，或其他unit文件，也可以参考这种形式，将测试对象传递至容器内部，并在子脚本中编写测试步骤和预期结果。

以上仅介绍了容器场景的测试框架和用例编写，随着测试场景的丰富，后续会跟进补充。

## FAQ

##

## 参考资料

1. [Rust程序设计语言 简体中文版 —— 编写自动化测试](http://kaisery.github.io/trpl-zh-cn/ch11-00-testing.html)
2. [Rust语言圣经 —— 自动化测试](http://course.rs/test/intro.html)
