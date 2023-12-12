# hwdb 手册

## 1. 简介
### 1.1 hwdb - 硬件数据库

硬件数据库是一个由 `modalias` 风格键和 `devmaster属性` 风格的值组成的 key-value 文本数据库。主要用于 `devmaster` 为匹配到的硬件设备添加关联属性，但也可以用于直接查询。

### 1.2 modalias介绍
modalias是Linux系统中的一种文件，存储了用于描述设备的属性和标识符。它通常位于/sys目录下的设备路径中，例如`/sys/devices/LNXSYSTM\:00/modalias`。

## 2. hwdb文件
### 2.1 hwdb文件读取路径
hwdb文件位于操作系统发行商维护的 `/usr/lib/devmaster/hwdb.d` 目录中，以及系统管理员维护的 `/etc/devmaster/hwdb.d` 目录中。注意：文件必须以 `.hwdb` 作为后缀名，目录中其他扩展名的文件将被忽略。

### 2.2 hwdb文件规则
每个hwdb文件都包含一系列由 `modalias字符串` 与关联的 `devmaster属性键值对` 组成的记录。每条记录都由一行或多行 `modalias字符串(可包含shell风格的通配符)` 开头，多个 `modalias字符串` 之间使用换行符分隔, 但必须是依次紧紧相连的行(也就是中间不能出现空行)，每一行都必须是一个完整的`modalias字符串` (也就是不能将一个 `modalias字符串` 分为两行)， 多行之间是逻辑或(OR)的关系。每一个 `modalias字符串` 都必须顶行书写(也就是行首不能是空白字符)。

`modalias字符串`可用的通配符：
- `"*"` ：匹配任意数量的字符
- `"?"` ：匹配单个字符
- `"[]"` ：匹配括号中的任意一个字符，比如 `[ab]` 可以匹配单个 `a` 或 `b` ，`[0-9]` 匹配任意个位数字。如果中括号中的内容以 `!` 开头，则匹配不属于中括号中内容的单个字符，比如 `[!a]` 表示匹配任意一个非 `a` 字符。

`modalias字符串` 后面跟一行或多行以空格开头 `devmaster属性键值对`。`devmaster属性键值对` 的书写必须符合 `key=value` 格式。最后通过空行表示一条记录的结束。`"#"`开头的行将被视为注释而忽略。

如果查询的 `modalias字符串` 匹配到了多个记录，那么记录中的 `devmaster属性键值对` 都会被获取。如果匹配到 `devmaster属性键值对` 中的 `key` 出现了多次，那么仅以最高优先级记录中的 `value` 为准(每个 `key` 仅允许拥有一个单独的 `value`)。对于不同hwdb文件中的记录来说，文件名的字典顺序越靠后，优先级越高；对于同一个hwdb文件中的记录来说， 记录自身的位置越靠后，优先级越高。

## 3. hwdb.bin文件
所有hwdb文件的数据都由 `sysmaster-hwdb` 读取并编译到 `/etc/devmaster/hwdb.bin` 或 `/usr/lib/devmaster/hwdb.bin` 中。在系统运行期间，仅会读取二进制格式的硬件数据库 `hwdb.bin` 而不会操作hwdb文件。

## 4. 示例
### 4.1 示例1：hwdb文件的一般语法
- /usr/lib/devmaster/hwdb.d/example.hwdb
```
# Comments can be placed before any records. This is a good spot
# to describe what that file is used for, what kind of properties
# it defines, and the ordering convention.

# A record with three matches and one property
mouse:*:name:*Trackball*:*
mouse:*:name:*trackball*:*
mouse:*:name:*TrackBall*:*
 ID_INPUT_TRACKBALL=1

# The rule above could be also be written in a form that
# matches Tb, tb, TB, tB:
mouse:*:name:*[tT]rack[bB]all*:*
 ID_INPUT_TRACKBALL=1

# A record with a single match and five properties
mouse:usb:v046dp4041:name:Logitech MX Master:*
 MOUSE_DPI=1000@166
 MOUSE_WHEEL_CLICK_ANGLE=15
 MOUSE_WHEEL_CLICK_ANGLE_HORIZONTAL=26
 MOUSE_WHEEL_CLICK_COUNT=24
 MOUSE_WHEEL_CLICK_COUNT_HORIZONTAL=14
```
### 4.2 示例2：devmaster属性值的覆盖
- /usr/lib/devmaster/hwdb.d/60-keyboard.hwdb
```
evdev:atkbd:dmi:bvn*:bvr*:bd*:svnAcer*:pn*:*
 KEYBOARD_KEY_a1=help
 KEYBOARD_KEY_a2=setup
 KEYBOARD_KEY_a3=battery

# Match vendor name "Acer" and any product name starting with "X123"
evdev:atkbd:dmi:bvn*:bvr*:bd*:svnAcer:pnX123*:*
 KEYBOARD_KEY_a2=wlan
```
- /etc/devmaster/hwdb.d/70-keyboard.hwdb
```
# disable wlan key on all at keyboards
evdev:atkbd:*
 KEYBOARD_KEY_a2=reserved
 PROPERTY_WITH_SPACES=some string
```
如果`hwdb.bin`仅由这两个文件组成，那么查找`modalias`值为`evdev:atkbd:dmi:bvn:bvr:bdXXXXX:bd08/05/2010:svnAcer:pnX123:`的键盘将匹配上述全部三条记录，并且最终获得如下"属性=值"：
```
    KEYBOARD_KEY_a1=help
    KEYBOARD_KEY_a2=reserved
    KEYBOARD_KEY_a3=battery
    PROPERTY_WITH_SPACES=some string
```
