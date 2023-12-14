#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <libudev.h>

void dump(struct udev_device *d)
{
	struct udev_list_entry *list_entry;
	const char *s = udev_device_get_syspath(d);

	printf("syspath: %s\n", s);

	udev_list_entry_foreach(list_entry, udev_device_get_devlinks_list_entry(d))
	{
		printf("link:      '%s'\n", udev_list_entry_get_name(list_entry));
	}

	udev_device_unref(d);
}

void main()
{
	while (1)
	{
		struct udev_device *lo = udev_device_new_from_device_id(NULL, "n1");
		dump(udev_device_ref(lo));
		lo = udev_device_unref(lo);

		/* Require /dev/sda1 exists and its device number is 8:1. */
		struct udev_device *sda1 = udev_device_new_from_device_id(NULL, "b8:1");
		dump(udev_device_ref(sda1));
		sda1 = udev_device_unref(sda1);

		struct udev_list_entry *list_entry;
		struct udev_enumerate *e = udev_enumerate_new(NULL);
		udev_enumerate_add_match_subsystem(e, "block");
		udev_enumerate_add_match_property(e, "ID_TYPE", "disk");
		udev_enumerate_add_match_is_initialized(e);
		udev_enumerate_scan_devices(e);
		udev_list_entry_foreach(list_entry, udev_enumerate_get_list_entry(e))
		{
			printf("block syspath:      '%s'\n", udev_list_entry_get_name(list_entry));
		}
		udev_enumerate_unref(e);
	}
}
