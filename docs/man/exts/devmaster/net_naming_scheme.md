# 网卡命名策略

由于`Linux`内核无法保证网卡的发现顺序，网卡设备的初始命名`ethN`无法映射到固定网卡。为了确保网卡名可以映射到固定的网卡设备上，`devmaster`实现了内置命令`net_id`，它会读取网卡的硬件属性，并根据网卡命名策略`naming scheme`，生成特定网卡设备上固定的`property`属性，基于这些属性，可以获得特定于网卡设备的命名。

在`net_id`生成这些环境变量并导入到`devmaster`的数据库后，`devmaster`的内置命令`net_link_setup`会根据网卡配置中的`NamePolicy`配置项，选择其中一个网卡属性，作为`ID_NET_NAME`属性的值。之后通过`NAME`赋值规则，将网卡重命名为`ID_NET_NAME`的值。

## 1. 命名策略设置

网卡命名策略用于控制`devmaster`生成的网卡名，`devmaster`提供了两种策略设置方法：

1. 配置启动参数: `net.naming-scheme=<scheme>`

2. 设置环境变量: `NET_NAMING_SCHEME=<scheme>`

环境变量`NET_NAMING_SCHEME`的值以':'开头时，优先使用启动参数中指定的策略，否则优先使用环境变量中指定的策略。命名策略主要影响内置命令`net_id`获取的网卡属性，同时会改变`net_setup_link`或规则处理过程中的网卡命名行为。设置为`v000`或`0`关闭网卡重命名功能。

为了保持向后兼容并提供策略可扩展能力，`devmaster`使用了策略开关机制。不同的命名策略由多种策略开关组合而成，新策略在原策略的基础上，新增一些开关。 **目前`devmaster`仅支持两种策略组合，`latest`策略和`udev`的253版本策略保持兼容，`v023`策略和`udev`的249版本保持兼容。未来会考虑针对`udev`不同版本的兼容性做更细粒度的划分。**

## 2. 网卡属性

内置命令`net_id`生成的网卡属性总是以2位英文字母作为前缀`prefix`，根据网卡类型的不同，属性前缀包括以下5类：

- `en`：以太网（Ethernet）
- `ib`：无限带宽（InfiniBand）

- `sl`：串列线路IP协议（Serial Line Internet Protocol）
- `wl`：无线局域网（Wireless local area network （WLAN））
- `ww`：无线广域网（Wireless wide area network （WWAN））

`devmaster`的内置命令`net_id`导出的`property`属性如下：

- `ID_NET_NAME_ONBOARD`：根据板载网卡固件提供的序列信息设置属性值，属性值受策略开关影响。支持的属性值如下：

|属性值|描述|涉及的策略开关|
|-|-|-|
|`<prefix>o<number>`|`number`为网卡的`PCI`板载索引，打开`ONBOARD_16BIT_INDEX`开关时支持16位索引，否则只支持14位索引。打开`ZERO_ACPI_INDEX`开关时，允许索引值为0。|ONBOARD_16BIT_INDEX、ZERO_ACPI_INDEX|
|`<prefix>d<number>`|`number`为网卡的`Devicetree`别名索引。|DEVICETREE_ALIASES|

- `ID_NET_LABEL_ONBOARD=<prefix><label>`：`label`为板载设备的固件提供的文本标签。该属性仅对`PCI`设备生效。如果`LABEL_NOPREFIX`开关打开，则不附加`prefix`前缀。
- `ID_NET_NAME_MAC=<prefix>x<mac address>`：`mac address`为12位的十六进制`MAC`地址。该属性仅在网卡拥有固定的`MAC`地址时生效。`MAC`地址和唯一的网卡设备绑定。
- `ID_NET_NAME_SLOT`：根据网卡的插槽位置设置属性值，不同类型网卡对应的属性值受不同的策略开关影响。根据网卡设备类型的不同，属性值中包含`USB`、`BCMA`、`SR-VIO`插槽号等信息。
|属性值|描述|涉及的策略开关|
|-|-|-|
|`<prefix>[P<domain>]s<slot>[f<function>][n<port_name>|d<dev_port>]`|`PCI`插槽号。当`PCI`的域号不为0时，附加`P<domain>`信息。当网卡为多功能`PCI`设备时，附加`f<function>`信息。如果网卡拥有端口名`port_name`时，附加`n<port_name>`信息，否则附加端口号信息`d<dev_port>`。|无|
|`<prefix>[P<domain>]s<slot>[f<function>][n<port_name>|d<dev_port>]b<number>`|`number`为`Broadcom bus（BCMA）`的核心号，如果为0，则舍弃该附加值。|无|
| `<prefix>[P<domain>]s<slot>[f<function>][n<port_name>|d<dev_port>]u<port...>[c<config>][i<interface>]`|`USB`端口号作为后缀。仅当开关打开，且网卡设备通过`USB`接口连接时生效。如果`USB`端口号超过15个字符，当`USB`配置号为1时，舍弃`c<config>`附加值。当接口号为`0`时，舍弃`i<interface>`附加值。|USB_HOST|
|`<prefix>[P<domain>]s<slot>[f<function>][n<port_name>|d<dev_port>]v<slot>`|如果网卡为`SR-IOV`虚拟设备，附加虚拟设备号后缀`v<slot>`。`slot`为虚拟设备号。|SR_IO_V|
|`<prefix>v<slot>`|`VIO`的插槽号（`IBM PowerVM`）。|无|
|`<prefix>X<number>`|`VIF`卡号（`Xen`）。|XEN_VIF|


- `ID_NET_NAME_PATH`：该属性描述了设备的安装位置。不同总线类型生成的属性值受不同策略开关的影响。对于`USB`和`BCMA`设备，属性值由网卡前缀、`PCI`插槽标识符和`USB`或`BCMA`位置信息组成。

|属性值|描述|策略开关|
|-|-|-|
| `<prefix>c<bus_id>`|`CCW`或组`CCW`设备标识符。|无|
| `<prefix>a<vencor model>i<instance>`|`arm64`平台设备的`ACPI`路径名。|无|
| `<prefix>i<address>n<port_name>`|模拟网络设备`Netsim`的设备号和端口号。|NETDEVSIM|
| `<prefix>[P<domain>]p<bus>s<slot>[f<function>][n<phys_port_name>|d<dev_port>]`|`PCI`的物理位置信息。|无|
| `<prefix>[P<domain>]p<bus>s<slot>[f<function>][n<phys_port_name>|d<dev_port>]b<number>`|`BCMA`网卡会附加`b<number>`后缀，`number`为`BCMA`总线核心号。|无|
| `<prefix>[P<domain>]p<bus>s<slot>[f<function>][n<phys_port_name>|d<dev_port>]u<port...>[c<config>][i<interface>]` |`USB`端口号作为后缀。`config`非1，`interface`非0。|USB_HOST|


## 策略开关

| 策略开关 | 描述 | 策略版本 |
| -------- | ---- |- |
| SR_IOV_V | 控制`net_id`内置命令，如果网卡为`SR-IOV`虚拟设备，`ID_NET_NAME_SLOT`属性中附加虚拟设备号后缀。 | v023 |
|NPAR_ARI|控制`net_id`内置命令，如果网卡开启了`ARI`模式，使用传统的5比特插槽号和3比特功能号组合成`f<function>`|v023|
|INFINIBAND|控制`net_id`内置命令，允许使用`ib`前缀。|v023|
|ZERO_ACPI_INDEX|控制`net_id`内置命令，允许`acpi_index`值为0，影响`ID_NET_NAME_ONBOARD`。|v023|
|ALLOW_RERENAMES|控制`net_setup_link`内置命令，允许`devmaster`重命名网卡。**`devmaster`尚不支持该开关的功能。**|v023|
|STABLE_VIRTUAL_MACS|控制`net_setup_link`内置命令，使用设备名生成`MAC`地址。**`devmaster`尚不支持该开关的功能。**|v023|
|NETDEVSIM|控制`net_id`内置命令，`ID_NET_NAME_PATH`以模拟网络设备`Netsim`的设备号和端口号作为后缀。|v023|
|LABEL_NOPREFIX|控制`net_id`内置命令，影响`ID_NET_LABEL_ONBOARD`的前缀附加条件。|v023|
|NSPAWN_LONG_HASH|控制`nspawn`，支持长整数哈希。 **`devmaster`不涉及。**|v023|
|BRIDGE_NO_SLOT|控制`net_id`内置命令，如果网卡是`PCI`桥设备，不使用`PCI`热插拔插槽号信息。|v023|
|SLOT_FUNCTION_ID|控制`net_id`内置命令，使用`function_id`属性标识`PCI`热插拔插槽位置，影响属性中的`s<slot>`后缀。|v023|
|ONBOARD_16BIT_INDEX|控制`net_id`内置命令，允许16比特的`acpi_index`索引值。|v023|
|REPLACE_STRICTLY|控制`devmaster`规则处理行为，不允许网卡名中存在非法字符。 **`devmaster`尚不支持该开关的功能。**|v023|
|XEN_VIF|控制`net_id`内置命令，如果网卡时`Xen`设备，使用`VIF`卡号作为`ID_NET_NAME_SLOT`的后缀。|latest|
|BRIDGE_MULTIFUNCTION_SLOT|控制`net_id`内置命令，开关`BRIDGE_NO_SLOT`打开时，如果网卡为多功能`PCI`设备，使用关联网桥的`PCI`热插拔插槽信息。|latest|
|DEVICETREE_ALIASES|控制`net_id`内置命令，以`Devicetree`别名生成`ID_NET_NAME_ONBOARD`。|latest|
|USB_HOST|控制`net_id`内置命令，使用`USB`端口号作为后缀。|latest|
