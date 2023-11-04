# 17 内置命令 usb_id

## 特性描述

usb_idb内置命令，用于读取usb设备信息

## 特性约束

无

## 子场景 1

测试读取usb设备信息

### 备注

无

### 准备工作

无

### 测试步骤

步骤1：使用devmaster的usb_id内置命令获取usb设备的属性
```
# devctl test-builtin usb_id /sys/bus/usb/devices/usb1
```

步骤2：使用udev的usb_id内置命令获取usb设备的属性
```
# udevadm test-builtin usb_id /sys/bus/usb/devices/usb1
```

### 结果验证

预期结果:步骤1和2中如下项应保持一致，不在此列表中的值不比较，
```
ID_BUS
ID_MODEL
ID_MODEL_ENC
ID_MODEL_ID
ID_SERIAL
ID_SERIAL_SHORT
ID_VENDOR
ID_VENDOR_ENC
ID_VENDOR_ID
ID_REVISION
ID_TYPE
ID_INSTANCE
ID_USB_MODEL
ID_USB_MODEL_ENC
ID_USB_MODEL_ID
ID_USB_SERIAL
ID_USB_SERIAL_SHORT
ID_USB_VENDOR
ID_USB_VENDOR_ENC
ID_USB_VENDOR_ID
ID_USB_REVISION
ID_USB_TYPE
ID_USB_INSTANCE
ID_USB_INTERFACES
ID_USB_INTERFACE_NUM
ID_USB_DRIVER
```

```
# devctl test-builtin usb_id /sys/bus/usb/devices/usb1
ID_BUS=usb
ID_MODEL=EHCI_Host_Controller
ID_MODEL_ENC=EHCI\x20Host\x20Controller
ID_MODEL_ID=0002
ID_SERIAL=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd_EHCI_Host_Controller_0000:02:03.0
ID_SERIAL_SHORT=0000:02:03.0
ID_VENDOR=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd
ID_VENDOR_ENC=Linux\x205.10.0-136.12.0.86.oe2203sp1.x86_64\x20ehci_hcd
ID_VENDOR_ID=1d6b
ID_REVISION=0510
ID_USB_MODEL=EHCI_Host_Controller
ID_USB_MODEL_ENC=EHCI\x20Host\x20Controller
ID_USB_MODEL_ID=0002
ID_USB_SERIAL=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd_EHCI_Host_Controller_0000:02:03.0
ID_USB_SERIAL_SHORT=0000:02:03.0
ID_USB_VENDOR=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd
ID_USB_VENDOR_ENC=Linux\x205.10.0-136.12.0.86.oe2203sp1.x86_64\x20ehci_hcd
ID_USB_VENDOR_ID=1d6b
ID_USB_REVISION=0510
ID_USB_INTERFACES=:090000:

# udevadm  test-builtin usb_id /sys/bus/usb/devices/usb1
ID_VENDOR=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd
ID_VENDOR_ENC=Linux\x205.10.0-136.12.0.86.oe2203sp1.x86_64\x20ehci_hcd
ID_VENDOR_ID=1d6b
ID_MODEL=EHCI_Host_Controller
ID_MODEL_ENC=EHCI\x20Host\x20Controller
ID_MODEL_ID=0002
ID_REVISION=0510
ID_SERIAL=Linux_5.10.0-136.12.0.86.oe2203sp1.x86_64_ehci_hcd_EHCI_Host_Controller_0000:02:03.0
ID_SERIAL_SHORT=0000:02:03.0
ID_BUS=usb
ID_USB_INTERFACES=:090000:
```

### 测试结束

无

### 测试场景约束

无
