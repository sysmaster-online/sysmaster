#include <stdio.h>
#include <unistd.h>
#include <signal.h>

void handler(int signum, siginfo_t *siginfo, void *ucontext){
    siginfo->si_code+=14;
    // follow systemd function
}

void install_handler(){
    struct sigaction act;
    act.sa_sigaction = handler;
    act.sa_flags = SA_SIGINFO;
    sigemptyset(&act.sa_mask);
    sigaction(SIGRTMIN+7, &act, NULL);
}

int main(){
    install_handler();
    for(int i=0;i<10;i++){
        sleep(100);
    }
}

