#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <libudev.h>

void dump(struct udev_device *d)
{
    struct udev_list_entry *list_entry;
    const char *s = udev_device_get_syspath(d);

    udev_list_entry_foreach(list_entry, udev_device_get_properties_list_entry(d))
    {
        printf("%s=%s\n", udev_list_entry_get_name(list_entry), udev_list_entry_get_value(list_entry));
    }
}

void main()
{
    /* Export environment variables before running this example:
     *
     * export SUBSYSTEM=net DEVPATH=/devices/virtual/net/lo SEQNUM=100 ACTION=add
     *
     * If the above environment variables are not exported, udev_device_new_from_environment
     * will fail.
     */
    struct udev_device *lo = udev_device_new_from_environment(NULL);
    if (lo == NULL)
    {
        printf("udev_device_new_from_environment failed\n");
    }
    else
    {
        dump(lo);
        lo = udev_device_unref(lo);
    }
}
