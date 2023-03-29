#include <systemd/sd-daemon.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

void main()
{
	printf("start test for service notify.\n");
	char *e;

	e = getenv("NOTIFY_SOCKET");
	if (!e)
	{
		printf("NOTIFY_SOCKET env is not set.\n");
		return;
	}

	printf("notify socket: %s.\n", e);
	int r = sd_notify(0, "READY=1");
	if (r < 0)
	{
		printf("send notify message failed: %d.\n", r);
		return;
	}
}
