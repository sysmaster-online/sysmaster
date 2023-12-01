#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <libudev.h>

void dump(struct udev_device *d)
{
	const char *s = udev_device_get_syspath(d);

	printf("%s\n", s);

	udev_device_unref(d);
}

void main()
{
	while (1)
	{
		struct udev_device *lo = udev_device_new_from_device_id(NULL, "n1");
		dump(udev_device_ref(lo));
		lo = udev_device_unref(lo);
	}
}
