# systemd单元文件配置解析


## 背景

systemd的单元文件采用ini风格的配置文件，为实现完全兼容ini风格配置文件，sysmaster采用自研的`unit_parser`配置解析库。

## unit_parser的基础定义

在unit_parser配置解析库中，我们将一个完整的unit配置文件称为`Unit`，一个Unit由多个`Section`组成，每个Section有包含多个`Entry`，例如：

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

该配置文件整体被称为Unit，`Unit`，`Service`，`Install`为Section。`Description`，`Before`为`Unit` Section的两个Entry；`Type`，`ExecStart`，`RemainAfterExit`为`Service` Section的三个Entry；`WantedBy`为`Install` Section的一个Entry。

通常，配置文件至少包含一个Section，而一个Section至少包含一个Entry。

## unit_parser提供的功能

unit_parser能够根据用户提供的配置文件，解析得到一个Unit结构体。基础的使用方法如下：

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

这里我们首先定义了`SysmasterUnit`结构体，这个结构体包含三个成员`Unit: SectionUnit`，`Service: SectionService`，`Install: SectionInstall`分别对应配置文件中的`Unit`，`Service`，`Install`三个Section。SectionUnit的每个成员分别对应`Unit` Section中的`Description`，`Before`，类似的，SectionService，SectionInstall与`Service`，`Install` Section中的Entry对应。

在main函数中，我们用config_path_vec构造解析地址，然后使用`SysmasterUnit::load_config`根据config_path_vec生成sysmaster_unit实体。

最后我们尝试打印一下Service Section的Type Entry，如果一切ok，那么终端将打印出“Type is: oneshot”。

在上面的最简示例中，我们存在以下四处特殊用法：

1. `load_config`：根据用户指定的配置文件，解析得到Unit实体。
2. `#[derive(UnitConfig)]`：声明当前定义的结构体为Unit，并为Unit实现相应的初始化、解析函数。
3. `#[derive(UnitSection)]`：声明顶前定义的结构体为Section，并为Section实现相应的初始化、解析函数。
4. `#[entry()]`：为各个Entry配置属性。

下面我们对这几点内容做具体介绍。

## 外部接口：load_config

`UnitConfig::load_config()`是unit_parser对外提供的唯一解析函数，第一个参数为解析文件路径数组、第二个参数为Unit完整名称。

* 解析文件路径数组：一个Unit实体可以通过不同的配置文件解析得到，因此这里将配置文件路径按照数组的形式组合。对于相同的Entry，在数组后面出现的配置文件具有更高优先级。
* Unit完整名称：某些Entry的解析依赖于Unit完整名称，因此用户必须传入完整的Unit名称。

## 过程宏：UnitConfig、UnitSection

UnitConfig、UnitSection是unit_parser定义的两个过程宏，两者的实现比较相似，我们首先介绍UnitConfig。

UnitConfig自动为结构体实现UnitConfig trait，该trait主要包括四个函数：

1. **load_config：** 对外提供的配置解析接口。
2. **__load：** load_config的内部具体实现。
1. **__load_default：** 负责解析结构体的初始化，加载缺省值。
2. **__parse_unit：** 根据用户给定的配置文件，解析得到Unit实体。

用户调用`UnitConfig::load_config()`，通过给定配置文件路径、Unit名称，解析得到Unit实体。

`load_config()`首先调用`__load_default()`加载缺省值，然后调用`__load()`加载用户的配置文件，`__load()`的加载过程会调用`__parse_unit()`。

## `#[entry()]`

unit_parser允许通过配置属性的方式为不同的Entry采用不同的解析策略。例如：

```rs
pub struct SectionService {
    #[entry(default = true)]
    pub RemainAfterExit: bool,
}
```

这里的意思是：RemainAfterExit的缺省值为`true`。如果用户的配置文件中没有配置RemainAfterExit，那么最终的Unit实体中，RemainAfterExit的值为`true`。

`#[entry()]`支持配置的属性如下：

* default: Entry的缺省值，如果不配置该属性，Entry的类型必须为Option<>
* append：Entry采用追加模式更新。允许该Entry配置多次，后配置的值将追加到之前解析的值上。类型为Vec<>
* prser：使用用户自定义的解析函数。
* key：Entry的别名。例如为RemainAfterExit配置`#[entry(key = "RemainWhenExit")]`，在配置文件中解析到`RemainWhenExit`时按照`RemainAfterExit`处理。

### 过程宏：UnitEntry

在前面的示例中，我们的每个Entry都是一些rust基础的数据类型，unit_parser为这些基础数据类型实现了UnitEntry trait。用户自定义的数据类型，需要手动实现UnitEntry trait，或者使用`parser`属性。

```rs
pub trait UnitEntry: Sized {
    type Error;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error>;
}
```

已经实现UnitEntry的数据类型包括：

* ​`bool`​​​：根据systemd.syntax的定义，`yes`​​​，`1`​​​，`on`​​​，`true`解析为`true`​；​`no`​​​，`0`​​​，`off`​​​，`false`​​​ 解析为`false`​​​。
* ​`chrono::Duration`​​​：根据systemd.time的定义解析。
* ​`Enum`​​：自定义的枚举类型，可以使用 `#[derive(UnitEntry)]`​​ 自动实现 `UnitEntry`​​。

## nom解析器

### 预处理

首先，通过编写PEG语法文件和使用`pest`​​库进行预解析，首先验证 Unit配置文件的基本结构是否合法。

解析后的内部状态被包装为`UnitParser`​​和`SectionParser`​​，其求值是惰性的，也不会产生额外的复制开销。`UnitParser`​​是一个返回`SectionParser`​​的迭代器，`SectionParser`​​是一个返回`(String, String)`​​键值对的迭代器。


## 参考文档

* [freedesktop.org - systemd.unit](https://www.freedesktop.org/software/systemd/man/systemd.unit.html "systemd.unit")
* [freedesktop.org - systemd.syntax](https://www.freedesktop.org/software/systemd/man/systemd.syntax.html# "systemd.syntax")
* [freedesktop.org - systemd.time](https://www.freedesktop.org/software/systemd/man/systemd.time.html# "systemd.time")
* [freedesktop.org - Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#recognized-keys "Desktop Entry Specification")

　　‍
