# 0001-systemd_parser

- submit: by j4ger on 2023-08-14

## 提供 Systemd Unit File 的解析方法

---

## 目的

　　	`systemd_parser`​ 库提供了可以解析 Systemd Unit File 和部分 XDG Desktop Entry 文件的宏和函数，只需使用 `#[derive]`​ 宏为指定的 Unit Spec 定义 Struct 实现核心 Trait，即可直接在程序逻辑中调用解析函数得到对应 Struct 结果。

## 详细设计

　　	首先，我们将一个 Systemd Unit File 的结构总结为 **Section** 和 **Entry**，其中：

* **Section ​**从一个由中括号包裹的 Section 名开头，到下一个 Section 头部或文件尾结束；
* Section 体部分由 **Entry ​**组成，具体形式为以 `=`​ 分隔的键值对。

　　	以下文档以此 Unit File 为例：

```ini
[Product]
Name=Outer Wilds
type=Game

[Game]
Platform=Windows Linux macOS # Linux support is done via Proton
Demo=yes
Price=45
AverageGameTime=20h30m 14s 13ms
# It does not actually support macOS
Executable=OuterWilds.exe
Executable=OuterWilds.appimage
Executable=OuterWilds.app
```

　　	该文件包含两个 Section：Product 和 Game，以及内部的多条 Entry。

　　	`systemd_parser`​ 的核心 Trait 对应了 Unit File 的三个部分：

* ​`UnitConfig`​​​：对应一类 Unit File，其中每个 Field 对应一个 Section，应当为实现了 `UnitSection`​​​ 的 Struct；
* ​`UnitSection`​​​：对应一类 Section，其中每个 Field 对应一个 Entry，应当为实现了 `UnitEntry`​​​ 的类型；
* ​`UnitEntry`​​：对应一个 Entry 键值对，其类型应当实现 `UnitEntry`​​ Trait，本质与 `std::str::FromStr`​​ 相同，库中已对默认实现 `std::str::FromStr`​​ 的类型实现了 `UnitEntry`​​，详见下一部分。

　　	使用时，需要添加 `systemd_parser`​ 依赖：

```sh
cargo add systemd_parser
```

　　	或在 `Cargo.toml`​ 的 `[dependencies]`​ 段落中添加：

```toml
systemd_parser = "0.1.0"
```

　　	并引入核心类型：

```rust
use systemd_parser::prelude::*;
```

　　	由此，针对上述 Unit File，编写的 Struct 定义如下：

```rust
#![allow(non_snake_case)]

use systemd_parser::prelude::*;

#[derive(UnitConfig, Debug)]
struct ProductUnit {
  Product: ProductSection,
  Game: GameSection,
}

#[derive(UnitSection, Debug)]
struct ProductSection {
  Name: String,
  Type: ProductType,
}

#[derive(UnitEntry, Debug)]
enum ProductType {
  Game, Software,
}

#[derive(UnitSection, Debug)]
struct GameSection {
  Platform: Vec<Platform>,
  Demo: bool,
  Price: u32,
  AverageGameTime: chrono::Duration,
  Executable: Vec<String>,
}

#[derive(UnitEntry, Debug)]
enum Platform {
  Windows, Linux, macOS,
}
```

　　	在主程序逻辑中，只需使用 `UnitConfig`​ 结构体上的 `load`​ 方法从文件读取，或使用 `load_from_string`​ 方法从字符串读取，如下：

```rust
let config = ProductUnit::load("/usr/share/appstore/OuterWilds.unit")?;
```

### 特殊标记

　　	在结构体定义中，可以使用特殊的 Attribute 来改变解析时的行为。

#### Section Attribute

　　	所有 Section Attribute 应用在 `UnitConfig`​ 结构体中的 Field 上，使用 `#[section()]`​ 作为外标记。

##### default

　　	指定对应 Section 的默认值，若文件中未找到该 Section 则使用默认值。对应 Section 结构体需要实现 `std::default::Default`​，即其默认值。

```rust
#[derive(UnitConfig)]
struct Unit {
  #[section(default)]
  Section: Section,
}

#[derive(UnitSection)]
struct Section {
  Entry: u64,
}

impl Default for Section {
  fn default() -> Self {
    Self { Entry: 0 }
  }
}
```

##### key

　　	指定对应 Section 的键名。默认使用 Field 名作为 Section 名在文件中寻找 Section 定义，可使用 `str`​ 形式的键名覆盖这一行为。

```rust
#[derive(UnitConfig)]
struct Unit {
  #[section(key = "AlternativeName")]
  Section: Section,
}

#[derive(UnitSection)]
struct Section {
  Entry: u64,
}
```

##### optional

　　	指定对应的 Section 为可选项。默认情况下，若无法找到一个 Section，解析逻辑会返回 `Error`​ 。若指定 `optional`​，对应 Field 会保留 `None`​ 值。`optional`​ Field 必须为 `Option<T>`​。

```rust
#[derive(UnitConfig)]
struct Unit {
  #[section(optional)]
  Section: Vec<Section>,
}

#[derive(UnitSection)]
struct Section {
  Entry: u64,
}
```

#### Entry Attribute

　　	所有 Entry Attribute 应用在 `UnitSection`​ 结构体中的 Field 上，使用 `#[entry()]`​ 作为外标记。

##### default

　　	指定对应 Entry 的默认值，若文件中未找到该 Entry 则使用默认值。需要传入一个 `Expr`​ 表达式。

```rust
#[derive(UnitConfig)]
struct Unit {
  Section: Section,
}

#[derive(UnitSection)]
struct Section {
  #[entry(default = 114)]
  Entry: u64,
}
```

##### key

　　	指定对应 Entry 的键名。默认使用 Field 名作为 Entry 名在文件中寻找 Entry 定义，可使用 `str`​ 形式的键名覆盖这一行为。

```rust
#[derive(UnitConfig)]
struct Unit {
  Section: Section,
}

#[derive(UnitSection)]
struct Section {
  #[entry(key = "AlternativeKey")]
  Entry: u64,
}
```

##### optional

　　	指定对应的 Entry 为可选项。默认情况下，若无法找到一个 Entry，解析逻辑会返回 `Error`​ 。若指定 `optional`​，对应 Field 会保留 `None`​ 值。`optional`​ Field 必须为 `Option<T>`​。

```rust
#[derive(UnitConfig)]
struct Unit {
  Section: Vec<Section>,
}

#[derive(UnitSection)]
struct Section {
  #[entry(optional)]
  Entry: Option<u64>,
}
```

##### multiple

　　	指定对应的 Entry 允许出现多次。默认情况下，最后一次出现的值会覆盖之前的值。指定 `multiple`​ 后，每次出现 Entry 时其值会被加入最终的 `Vec`​ 中。`multiple`​ Field 必须为 `Vec`​。

```rust
#[derive(UnitConfig)]
struct Unit {
  Section: Vec<Section>,
}

#[derive(UnitSection)]
struct Section {
  #[entry(multiple)]
  Entry: Vec<u64>,
}
```

#### Entry 类型

　　	`UnitEntry`​ Trait 已为所有实现 `std::str::FromStr`​ 的类型完成实现，此外特殊实现包括：

* ​`bool`​​：根据 systemd.syntax 中的定义，`yes`​​ `1`​​ `on`​​ `true`​​ 都被认为是 `true`​​，`no`​​ `0`​​ `off`​​ `false`​​ 都被认为是 `false`​​；
* ​`chrono::Duration`​​：根据 systemd.time 中的定义解析；
* ​`Vec<T>`​：若不指定 `multiple`​，则按照空格分割字符串，再依次解析为值，其中 `T`​ 必须实现 `UnitEntry`​；
* ​`Enum`​：自定义的枚举类型，可以使用 `#[derive(UnitEntry)]`​ 自动实现 `UnitEntry`​。

### 底层设计

#### 预处理

　　	首先，通过编写 PEG 语法文件和使用 `pest`​ 库进行预解析，首先验证 Unit File 的基本结构是否合法。

　　	解析后的内部状态被包装为 `UnitParser`​ 和 `SectionParser`​，其求值是惰性的，也不会产生额外的复制开销。`UnitParser`​ 是一个返回 `SectionParser`​ 的迭代器，`SectionParser`​ 是一个返回 `(String, String)`​ 键值对的迭代器。

#### 宏与代码生成

　　	`systemd_parser_macro`​ 库使用 `syn`​ 库解析 `#[derive()]`​ 修饰的结构体定义，处理其键名、类型和所加的 Attribute。对每个 Unit 的解析逻辑（其上生成的 `__parse_unit()`​ 方法）如下：

* 对每个键初始化同名变量，值为 `None`​；
* 调用 `systemd_parser`​ 函数获得 `UnitParser`​，迭代其中每一个 `SectionParser`​，用 `match`​ 语句匹配其键名（或 `default`​ Attribute 指定的键名），并调用对应 Section 结构体的 `__parse_section`​ 方法解析 `SectionParser`​。若解析成功，将同名变量设置为 `Some(section)`​；若解析报错，且未定义 `default`​，则报错；若有定义 `default`​ 则使用默认值；
* 在迭代完毕后对每个同名变量使用 `ok_or`​ （若无 `default`​ 定义）或 `unwrap_or`​ （若有 `default`​ 定义）得到内部值；若为 `optional`​，则跳过；
* 构造并返回 `Self`​。

　　	类似地，每个 Section 的解析逻辑（其上生成的 `__parse_section`​ 方法）如下：

* 对每个键初始化同名变量，值为 `None`​，若为 `multiple`​，则初始化为 `Vec::new()`​；
* 迭代 `SectionParser`​ 其中每一个 `(String, String)`​ 键值对，用 `match`​ 语句匹配其键名（或 `default`​ Attribute 指定的键名），并调用对应 Entry 结构体的 `parse_from_str`​ 方法解析。若解析成功，将同名变量设置为 `Some(entry)`​；若解析报错，且未定义 `default`​，则报错；若有定义 `default`​ 则使用默认值；若为 `multiple`​，则使用 `Vec::push()`​ 方法加入数组；
* 在迭代完毕后对每个同名变量使用 `ok_or`​ （若无 `default`​ 定义）或 `unwrap_or`​ （若有 `default`​ 定义）得到内部值；若为 `optional`​，则跳过；
* 构造并返回 `Self`​。

## 特殊事项

- XDG Desktop Entry 规范中定义了 locale 功能，在此并未实现；


## 当前进展

- 全部上述功能

## 待完成

- 英文文档
- 注释

## 参考文档

* [freedesktop.org - systemd.unit](https://www.freedesktop.org/software/systemd/man/systemd.unit.html "systemd.unit")
* [freedesktop.org - systemd.syntax](https://www.freedesktop.org/software/systemd/man/systemd.syntax.html# "systemd.syntax")
* [freedesktop.org - systemd.time](https://www.freedesktop.org/software/systemd/man/systemd.time.html# "systemd.time")
* [freedesktop.org - Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#recognized-keys "Desktop Entry Specification")

　　‍


