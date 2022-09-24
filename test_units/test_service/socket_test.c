#include<stdio.h>
#include <sys/socket.h>
#include <unistd.h>

void main() {
	int i, n;
	n = sd_listen_fds(1);
	int set_ret;
	int sendbuf = 0;

	for (i=3; i< n+3; i++) {
		socklen_t opt_len = sizeof(sendbuf);
		set_ret = getsockopt(i, SOL_SOCKET, SO_SNDBUF, (int *)&sendbuf, &opt_len);
		if(set_ret < 0) {
			continue;
		}

		printf("get send buffer: %d\n", sendbuf);

		set_ret = getsockopt(i, SOL_SOCKET, SO_SNDBUF, (int *)&sendbuf, &opt_len);
		if(set_ret < 0) {
			continue;
		}

		printf("get send buffer: %d\n", sendbuf);

	}


	printf("listend fds: %d\n", n);
}
