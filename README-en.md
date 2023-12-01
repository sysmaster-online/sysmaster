ysMaster （[http://sysmaster.online](http://sysmaster.online)）

## challenge

Process 1 is at a critical position in the system, responsible for system initialization and runtime service management, and is currently facing the following challenges:

- **(1) Poor reliability:** The location is critical, and the operating system must be restarted to recover when it fails
- **(2) High complexity:** systemd became the de facto standard for process No. 1. It introduces many new concepts and tools, which are complicated and difficult to tailor for actual use cases
- **(3) Weak compatibility:** Poor support for cloud, edge, and embedded scenarios, which does not meet the requirements of all scenarios

**Industry demands**

- **Server and embedded scenarios have strong requirements for reliability:** Memory safety issues are reduced to 0: Memory problems that are difficult to locate are eliminated and the impact of downtime is reduced
- **No system downtime, no business impact:** Pursue ultimate reliability with the never-downtime feature
- **Pain points in efficient and intelligent O&M scenarios:** Lack of efficient means for host O&M: Containerized OS proposes a simplified goal for process 1
- **Localization substitution requires independent and controllable core software:** Focus on key open source software: sensitive/competitive root components, and realize complete autonomy and controllability of root technology

## vision

1. Short-term competitiveness: Extreme reliability and system start-up speed

- **Never downtime:** Technologies such as external status and savepoint are used to achieve second-level self-healing of faults to ensure that process No. 1 is continuously online and services are not affected
- **Fast startup:** Deduplication and simplified mirroring are decoupled to optimize the server restart time from 3-5 minutes to 1 minute (5 minutes for competitors)

2. Long-term competitiveness: Improve the O&M efficiency of all-scenario applications and cloud platforms

- **All-scenario applications:** Supports scenarios such as cloud-edge-end, unified init, and supports bare metal, virtual machine, and containers
- **Containerized OS:** kernel+sysMaster = container OS base, which is interconnected with k8s andopenstack to support X times improvement of O&M efficiency in cloud scenarios together with KubeOS

Overall strategy: Focus on the system with the smallest function, adopt the parallel strategy of core self-developed and non-core community solutions, and the strategy of drawer replacement of independent components to achieve systemd self-developed substitution

1. Scenario-by-scenario: Combine different usage scenarios to split the systemd function into multiple layers, and gradually complete the function development, verification, and replacement according to the scenario from easy to difficult.

2. Multi-solution: The core functions and components of the non-community alternative are replaced by self-developed and rewritten, and the others are replaced by community software without supply risk.

3. Rhythmic: one core competitiveness a year, to 24 years to complete the exploration of three corecompetitiveness, according to the development - landing - substitution of the three lines of work.

## sysMaster: A new No. 1 process implementation solution, self-healing of second-level faults, ensuring that the system is online all day

sysMaster aims to improve the traditional init daemon, 1+1+N architecture.

- init: The new No. 1 process, with minimalist functions, 1,000 lines of code, and extreme reliability

- core: undertakes the original core functions of systemd, introduces a reliability framework and plug-in mechanism, and makes it capable of rapid self-healing of crashes, hot upgrades, and flexible assembly

- exts: makes the functions of each component of the original coupling independent, supports the replacement of the corresponding components of the systemd in a drawer style, and supports rhythmic substitution of sub-scenes

**Strive to build three major competitiveness:**

1. Extremely reliable –** never downtime**

- Simplified architecture: 1+1+N, simplifying INIT; Non-core functions are provided in a componentized manner
- Extremely reliable: fault detection + second-level recovery, and the root process is continuously online
- Memory safety: The memory problem is reduced to 0 and the fault is self-healing

2. Extremely lightweight –**fast start-up**

- Fewer resources: 10% lower memory footprint
- Faster speed: 15% faster startup

3. Superior Experience – Minimalist mirroring

- Easy O&M: Hot upgrades, on-demand tailoring, and convenient deployment/O&M
- Compatible ecosystem: Provide systemd ecosystem compatibility and conversion tools
- Plug-in mechanism: Supports flexible expansion of multiple service types

## construct

First, download the repository code and run the command to pre-install the project dependencies and build the development environment, the project is mainly built on Rust 1.57, and use pre-commit to do the git commit check.

```
sh ./build.sh
```

Second, the program can be built through the scripts provided. You can also refer to Action Building .pre-commit-config.yaml in .

```
sh ci/01-pre-commit.sh

# 格式检查
cargo clippy -vvv --all-targets --features "default" --all -- -Dwarnings

# 构建
cargo build --all --features "default" -v

# 测试
RUST_BACKTRACE=full cargo test --all-targets --all -v -- --nocapture --show-output --test-threads=1
```

## use

For more information about how to use it in various scenarios,please refer to the tools directory.

```
ls tools
```
You can also visit, sysmaster.online the official website for examples

## Description of the code directory structure

The source code repository is managed in the form of workspaces, each directory is a package, each package contains a crate (lib or bin form), the public lib crate directory is prefixed with lib, created with cargo new --lib libtests, and the bin crate directory of daemon type ends with d.

 ```text
/ (sysmaster)
|...init (init进程)
|...factory (系统配置)
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
|...build.sh (准备环境)
```

As:

```text
  - lib crate: libs/event, libs/basic
  - bin crate: extends/init, sysmaster
  - daemon crate: extends/udevd, extends/logind
```
