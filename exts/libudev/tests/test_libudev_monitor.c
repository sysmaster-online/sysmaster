#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <libudev.h>

void main()
{
    struct udev_monitor *m = udev_monitor_new_from_netlink(NULL, "kernel");
    udev_monitor_filter_add_match_subsystem_devtype(m, "block", "partition");
    udev_monitor_enable_receiving(m);
    while (1)
    {
        struct udev_device *d = udev_monitor_receive_device(m);
        if (d != NULL)
        {
            printf("%s\n", udev_device_get_syspath(d));
            udev_device_unref(d);
        }
    }
}
