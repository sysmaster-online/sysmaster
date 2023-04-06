/*
 * Description: fork two child processes, write the first pid to /run/pidfile
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
        return 0;
    } else if (pid > 0) {
        FILE *fp = NULL;
        fp = fopen(argv[2], "w");
        fprintf(fp, "%d\n", pid);
    } else {
        printf("Can't fork!");
    }

    // fork another child process, not main process
    pid = fork();
    if (pid == 0) {
        sleep(sec);
        return 0;
    }

    return 0;
}
