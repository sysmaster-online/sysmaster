/*
 * Description: fork child process, write pid to specific pidfile
 */

#include <stdio.h>
#include <unistd.h>
#include <sys/types.h>
#include <stdlib.h>

int main(int argc, char *argv[])
{
    int sec = atoi(argv[1]);
    pid_t pid = fork();

    if (pid == 0) {
        sleep(sec);
    } else if (pid > 0) {
        FILE *fp = NULL;
        fp = fopen(argv[2], "w");
        fprintf(fp, "%d\n", pid);
    } else {
        printf("Can't fork!");
    }

    return 0;
}
