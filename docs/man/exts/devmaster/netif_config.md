# 网卡配置

网卡配置用于控制内置命令`net_setup_link`的行为。`devmaster`无默认的网卡配置加载路径，需要在配置文件中显示指定，具体可参考`devmaster`手册。网卡配置文件采用`toml`格式，以`.link`作为文件后缀，按文件名的字典序加载。 **和`udev`不同，`devmaster`尚不支持同名网卡配置文件覆盖和配置文件的`dropin`**。

## 1. 配置文件

网卡配置文件中包含匹配节和控制节。匹配节中包含若干匹配项，当网卡满足所有匹配项条件时，执行控制节中的所有控制项，如设置网卡名、调整网卡参数等等。

### 匹配节

- `OriginalName`：匹配网卡的内核名`sysname`，支持`shell glob`类型的模式匹配。

### 控制节

- `NamePolicy`：可以配置一组网卡命名选项，`net_setup_link`会按配置顺序依次检查各个选项是否可用，如果找到了可用项，会将该选项对应的`property`属性赋给`ID_NET_NAME`属性。`net_setup_link`不会直接修改网卡名，需要通过`NAME`赋值规则，将网卡名改为`ID_NET_NAME`的值。
  - `database`：对应`ID_NET_NAME_FROM_DATABASE`属性，从硬件数据库`hwdb`中获取。 **当前`devmaster`尚不支持`hwdb`，该选项不生效。**
  - `onboard`：对应`ID_NET_NAME_ONBOARD`属性，由`net_id`内置命令基于板载网卡信息生成。
  - `slot`：对应`ID_NET_NAME_SLOT`属性，由`net_id`内置命令基于热插拔网卡设备的固件信息生成。
  - `path`：对应`ID_NET_NAME_PATH`属性，由`net_id`内置命令基于网卡的物理位置信息生成。
  - `mac`：对应`ID_NET_NAME_MAC`属性，由`net_id`内置命令基于网卡的`mac`地址生成。

## 参考案例

`devmaster`提供了默认网卡配置：

```toml
[Match]
OriginalName = "*"

[Link]
NamePolicy = ["database", "onboard", "slot", "path"]
```
