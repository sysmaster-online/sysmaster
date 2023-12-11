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
	}
}
