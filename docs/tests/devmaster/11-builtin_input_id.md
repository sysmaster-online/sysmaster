# 11 内置命令 input_id

## 特性描述

input_id内置命令，用于读取输入设备的属性

## 特性约束

无

## 子场景 1

测试读取输入设备的属性

### 备注

无

### 准备工作

无

### 测试步骤

步骤1：使用devmaster的input_id内置命令读取输入设备的属性
```
# devctl test-builtin input_id /sys/class/input/input* （*为数字编号）
```

步骤2：使用udev的input_id内置命令读取输入设备的属性
```
# udevadm test-builtin input_id /sys/class/input/input*
```

### 结果验证

预期结果:步骤1和2中如下项应保持一致，不在此列表中的值不比较
```
ID_INPUT
ID_INPUT_KEY
ID_INPUT_SWITCH
ID_INPUT_ACCELEROMETER
ID_INPUT_POINTINGSTICK
ID_INPUT_MOUSE
ID_INPUT_TOUCHPAD
ID_INPUT_TOUCHSCREEN
ID_INPUT_JOYSTICK
ID_INPUT_TABLET
ID_INPUT_TABLET_PAD
ID_INPUT_WIDTH_MM
ID_INPUT_HEIGHT_MM
ID_INPUT_KEYBOARD
```

```
# devctl test-builtin input_id /sys/class/input/input0
ID_INPUT=1
ID_INPUT_KEY=1

# udevadm test-builtin input_id /sys/class/input/input0
ID_INPUT=1
ID_INPUT_KEY=1
```

### 测试结束

无

### 测试场景约束

无
