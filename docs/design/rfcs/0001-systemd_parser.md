# systemd 单元文件配置解析

## 背景

systemd 的单元文件采用 ini 风格的配置文件，为实现完全兼容 ini 风格配置文件，sysmaster 采用自研的`unit_parser`配置解析库。

## unit_parser 的基础定义

在 unit_parser 配置解析库中，我们将一个完整的 unit 配置文件称为`Unit`，一个 Unit 由多个`Section`组成，每个 Section 有包含多个`Entry`，例如：

```ini
# ./test.service
[Unit]
Description=Test Service
Before=multi-user.target

[Service]
Type=oneshot
ExecStart=/bin/true
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

该配置文件整体被称为 Unit，`Unit`，`Service`，`Install`为 Section。`Description`，`Before`为`Unit` Section 的两个 Entry；`Type`，`ExecStart`，`RemainAfterExit`为`Service` Section 的三个 Entry；`WantedBy`为`Install` Section 的一个 Entry。

通常，配置文件至少包含一个 Section，而一个 Section 至少包含一个 Entry。

## unit_parser 提供的功能

unit_parser 能够根据用户提供的配置文件，解析得到一个 Unit 结构体。基础的使用方法如下：

```rs
use std::path::PathBuf;

use unit_parser::prelude::{UnitConfig, UnitSection};

#[derive(UnitConfig, Default)]
pub struct SysmasterUnit {
    pub Unit: SectionUnit,
    pub Service: SectionService,
    pub Install: SectionInstall,
}

#[derive(UnitSection, Default)]
pub struct SectionUnit {
    #[entry(default = String::new())]
    pub Description: String,
    #[entry(append)]
    pub Before: Vec<String>,
}

#[derive(UnitSection, Default)]
pub struct SectionService {
    #[entry(default = String::new())]
    pub Type: String,
    #[entry(default = String::new())]
    pub ExecStart: String,
    #[entry(default = true)]
    pub RemainAfterExit: bool,
}

#[derive(UnitSection, Default)]
pub struct SectionInstall {
    #[entry(append)]
    pub WantedBy: Vec<String>,
}

fn main() {
    let config_path_vec = vec![
        PathBuf::from("./test.service"),
    ];
    let sysmaster_unit = SysmasterUnit::load_config(config_path_vec, "test.service").unwrap();
    println!("Type is: {}", sysmaster_unit.Service.Type)
}
```

这里我们首先定义了`SysmasterUnit`结构体，这个结构体包含三个成员`Unit: SectionUnit`，`Service: SectionService`，`Install: SectionInstall`分别对应配置文件中的`Unit`，`Service`，`Install`三个 Section。SectionUnit 的每个成员分别对应`Unit` Section 中的`Description`，`Before`，类似的，SectionService，SectionInstall 与`Service`，`Install` Section 中的 Entry 对应。

在 main 函数中，我们用 config_path_vec 构造解析地址，然后使用`SysmasterUnit::load_config`根据 config_path_vec 生成 sysmaster_unit 实体。

最后我们尝试打印一下 Service Section 的 Type Entry，如果一切 ok，那么终端将打印出“Type is: oneshot”。

在上面的最简示例中，我们存在以下四处特殊用法：

1. `load_config`：根据用户指定的配置文件，解析得到 Unit 实体。
2. `#[derive(UnitConfig)]`：声明当前定义的结构体为 Unit，并为 Unit 实现相应的初始化、解析函数。
3. `#[derive(UnitSection)]`：声明顶前定义的结构体为 Section，并为 Section 实现相应的初始化、解析函数。
4. `#[entry()]`：为各个 Entry 配置属性。

下面我们对这几点内容做具体介绍。

## 外部接口：load_config

`UnitConfig::load_config()`是 unit_parser 对外提供的唯一解析函数，第一个参数为解析文件路径数组、第二个参数为 Unit 完整名称。

- 解析文件路径数组：一个 Unit 实体可以通过不同的配置文件解析得到，因此这里将配置文件路径按照数组的形式组合。对于相同的 Entry，在数组后面出现的配置文件具有更高优先级。
- Unit 完整名称：某些 Entry 的解析依赖于 Unit 完整名称，因此用户必须传入完整的 Unit 名称。

## 过程宏：UnitConfig、UnitSection

UnitConfig、UnitSection 是 unit_parser 定义的两个过程宏，两者的实现比较相似，我们首先介绍 UnitConfig。

UnitConfig 自动为结构体实现 UnitConfig trait，该 trait 主要包括四个函数：

1. **load_config：** 对外提供的配置解析接口。
2. **\_\_load：** load_config 的内部具体实现。
3. **\_\_load_default：** 负责解析结构体的初始化，加载缺省值。
4. **\_\_parse_unit：** 根据用户给定的配置文件，解析得到 Unit 实体。

用户调用`UnitConfig::load_config()`，通过给定配置文件路径、Unit 名称，解析得到 Unit 实体。

`load_config()`首先调用`__load_default()`加载缺省值，然后调用`__load()`加载用户的配置文件，`__load()`的加载过程会调用`__parse_unit()`。

## `#[entry()]`

unit_parser 允许通过配置属性的方式为不同的 Entry 采用不同的解析策略。例如：

```rs
pub struct SectionService {
    #[entry(default = true)]
    pub RemainAfterExit: bool,
}
```

这里的意思是：RemainAfterExit 的缺省值为`true`。如果用户的配置文件中没有配置 RemainAfterExit，那么最终的 Unit 实体中，RemainAfterExit 的值为`true`。

`#[entry()]`支持配置的属性如下：

- default: Entry 的缺省值，如果不配置该属性，Entry 的类型必须为 Option<>
- append：Entry 采用追加模式更新。允许该 Entry 配置多次，后配置的值将追加到之前解析的值上。类型为 Vec<>
- prser：使用用户自定义的解析函数。
- key：Entry 的别名。例如为 RemainAfterExit 配置`#[entry(key = "RemainWhenExit")]`，在配置文件中解析到`RemainWhenExit`时按照`RemainAfterExit`处理。

### 过程宏：UnitEntry

在前面的示例中，我们的每个 Entry 都是一些 rust 基础的数据类型，unit_parser 为这些基础数据类型实现了 UnitEntry trait。用户自定义的数据类型，需要手动实现 UnitEntry trait，或者使用`parser`属性。

```rs
pub trait UnitEntry: Sized {
    type Error;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error>;
}
```

已经实现 UnitEntry 的数据类型包括：

- ​`bool`​​​：根据 systemd.syntax 的定义，`yes`​​​，`1`​​​，`on`​​​，`true`解析为`true`​；​`no`​​​，`0`​​​，`off`​​​，`false`​​​ 解析为`false`​​​。
- ​`chrono::Duration`​​​：根据 systemd.time 的定义解析。
- ​`Enum`​​：自定义的枚举类型，可以使用 `#[derive(UnitEntry)]`​​ 自动实现 `UnitEntry`​​。

## nom 解析器

### 预处理

首先，通过编写 PEG 语法文件和使用`pest`​​ 库进行预解析，首先验证 Unit 配置文件的基本结构是否合法。

解析后的内部状态被包装为`UnitParser`​​ 和`SectionParser`​​，其求值是惰性的，也不会产生额外的复制开销。`UnitParser`​​ 是一个返回`SectionParser`​​ 的迭代器，`SectionParser`​​ 是一个返回`(String, String)`​​ 键值对的迭代器。

## 参考文档

- [freedesktop.org - systemd.unit](https://www.freedesktop.org/software/systemd/man/systemd.unit.html "systemd.unit")
- [freedesktop.org - systemd.syntax](https://www.freedesktop.org/software/systemd/man/systemd.syntax.html# "systemd.syntax")
- [freedesktop.org - systemd.time](https://www.freedesktop.org/software/systemd/man/systemd.time.html# "systemd.time")
- [freedesktop.org - Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#recognized-keys "Desktop Entry Specification")

‍
