#ifndef PCI_DRIVER_H
#define PCI_DRIVER_H

#include <stdint.h>

#define PCI_CONFIG_ADDRESS_PORT 0xCF8
#define PCI_CONFIG_DATA_PORT    0xCFC

#define PCI_DEVICE_HEADER_TYPE_DEVICE               0x00
#define PCI_DEVICE_HEADER_TYPE_PCI_TO_PCI_BRIDGE    0x01
#define PCI_DEVICE_HEADER_TYPE_CARDBUS_BRIDGE       0x02

// https://gist.github.com/cuteribs/0a4d85f745506c801d46bea22b554f7d
#define PCI_VENDOR_NONE         0xFFFF
#define PCI_VENDOR_INTEL        0x8086
#define PCI_VENDOR_REALTEK      0x10EC
#define PCI_VENDOR_QEMU         0x1234

#define PCI_DEVICE_ID_INTEL_82441       0x1237
#define PCI_DEVICE_ID_INTEL_82371SB_0   0x7000
#define PCI_DEVICE_ID_INTEL_82371SB_1   0x7010
#define PCI_DEVICE_ID_INTEL_82371AB_3   0x7113
#define PCI_DEVICE_ID_QEMU_VGA          0x1111
#define PCI_DEVICE_ID_REALTEK_8139      0x8139
#define PCI_DEVICE_ID_NONE              0xFFFF

// http://my.execpc.com/~geezer/code/pci.c
#define PCI_DEVICE_CLASS_DISK_CONTROLLER    0x01
#define PCI_DEVICE_CLASS_NETWORK_CONTROLLER 0x02
#define PCI_DEVICE_CLASS_DISPLAY_CONTROLLER 0x03
#define PCI_DEVICE_CLASS_BRIDGE             0x06

#define PCI_DEVICE_SUBCLASS_DISK_CONTROLLER_IDE         0x01
#define PCI_DEVICE_SUBCLASS_NETWORK_CONTROLLER_ETHERNET 0x00
#define PCI_DEVICE_SUBCLASS_DISPLAY_CONTROLLER_VGA      0x00
#define PCI_DEVICE_SUBCLASS_BRIDGE_CPU                  0x00
#define PCI_DEVICE_SUBCLASS_BRIDGE_ISA                  0x01
#define PCI_DEVICE_SUBCLASS_BRIDGE_OTHER                0x80

#endif