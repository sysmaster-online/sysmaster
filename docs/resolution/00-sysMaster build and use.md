# sysMaster 构建和使用

## 构建

首先，下载仓库代码，并执行命令来预装项目依赖，构建开发环境， 项目主要基于rust 1.57构建，使用``pre-commit`做git commit检查。
```
sh ./requirements.sh
```
其次，可以通过提供的脚本来构建程序。也可以参考`.pre-commit-config.yaml`中的动作构建。
```
# 脚本统一构建
sh ci/01-pre-commit.sh

# 格式检查
cargo clippy -vvv --all-targets --features "default" --all -- -Dwarnings

# 构建
cargo build --all --features "default" -v

# 测试
RUST_BACKTRACE=full cargo test --all-targets --all -v -- --nocapture --show-output --test-threads=1
```
## 使用

在各场景下的使用，可以参考源码仓库`tools`目录下内容, 部分场景提供了自动化的工具。[sysmaster源码仓库](https://gitee.com/openeuler/sysmaster/tree/master/tools)
```
musl-build
run_with_busybox
run_with_kubeos
run_with_sd
run_with_vm
```

## 代码目录结构说明

源码仓库以workspaces方式管理，每一个目录是一个package，每个package包含一个crate（lib或bin形式），
公共lib crate的目录带lib前缀，使用cargo new --lib libtests创建,
daemon类型的bin crate的目录以d结尾。

```text
/ (init)
|...libs (对外接口)
|     |...libtests (test lib crate)
|     |...cgroup (cgroup lib crate)
|     |...cmdproto(cmd proto lib crate)
|...exts (sysmaster-extends组件)
|     |...devmaster (daemon)
|     |...random-seed (bin)
|...core (sysmaster-core核心组件)
|     |...sysmaster (bin)
|     |...libcore (internal lib)
|     |...sctl (sysmaster cli)
|     |...coms (插件)
|          |...service (unit type crate)
|          |...socket  (unit type crate)
|          |...target  (unit type crate)
|...tools
|     |...musl_build
|     |...run_with_sd
|...docs (sysmaster.online)
|...requirements.sh (准备环境)
```

如：

```text
  - lib crate: libs/event, libs/basic
  - bin crate: extends/init, sysmaster
  - daemon crate: extends/udevd, extends/logind
```
