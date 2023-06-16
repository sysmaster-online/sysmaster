# sysmaster的信号处理

sysMaster采用`1+1+N`架构，sysmaster不再作为1号进程，因此内核为1号进程的信号做的特殊处理不再生效。下表列出了init(1号进程)、sysmaster、systemd的信号处理方式以及sysMaster与systemd针对不同信号的响应逻辑差异。

|信号|init|sysmaster|systemd|sysmaster与systemd对外表现是否有差异|
|-|-|-|-|-|
|1) SIGHUP|捕获不处理|daemon-reload|daemon-reload|N|
|2) SIGINT|捕获不处理|start ctrl-alt-del.target|start ctrl-alt-del.target|N|
|3) SIGQUIT|捕获不处理|故障恢复|crash handler|Y|
|4) SIGILL|捕获不处理|故障恢复|crash handler|Y|
|5) SIGTRAP|捕获不处理|IGN|DFL|N|
|6) SIGABRT|捕获不处理|1号进程下发重执行/故障恢复|crash handler|Y|
|7) SIGBUS|捕获不处理|故障恢复|crash handler|Y|
|8) SIGFPE|捕获不处理|故障恢复|crash handler|Y|
|9) SIGKILL|内核主动屏蔽该信号|DFL|内核主动屏蔽该信号|Y|
|10) SIGUSR1|捕获不处理|IGN|重连dbus|Y|
|11) SIGSEGV|捕获不处理|故障恢复|crash handler|Y|
|12) SIGUSR2|捕获不处理|IGN|输出所有单元的配置信息|Y|
|13) SIGPIPE|捕获不处理|IGN|IGN|N|
|14) SIGALRM|捕获不处理|IGN|DFL|N|
|15) SIGTERM|捕获不处理|daemon-reexec|daemon-reexec|N|
|16) SIGSTKFLT|捕获不处理|IGN|DFL|N|
|17) SIGCHLD|僵尸子进程回收|僵尸子进程回收|僵尸子进程回收|N|
|18) SIGCONT|捕获不处理|IGN|DFL|N|
|19) SIGSTOP|内核主动屏蔽该信号|DFL|内核主动屏蔽该信号|Y|
|20) SIGTSTP|捕获不处理|IGN|DFL|N|
|21) SIGTTIN|捕获不处理|IGN|DFL|N|
|22) SIGTTOU|捕获不处理|IGN|DFL|N|
|23) SIGURG|捕获不处理|IGN|DFL|N|
|24) SIGXCPU|捕获不处理|IGN|DFL|N|
|25) SIGXFSZ|捕获不处理|IGN|DFL|N|
|26) SIGVTALRM|捕获不处理|IGN|DFL|N|
|27) SIGPROF|捕获不处理|IGN|DFL|N|
|28) SIGWINCH|捕获不处理|IGN|start kbrequest.target|Y|
|29) SIGIO|捕获不处理|IGN|DFL|N|
|30) SIGPWR|捕获不处理|IGN|start sigpwr.target|Y|
|31) SIGSYS|捕获不处理|故障恢复|DFL|N|
|34) SIGRTMIN|捕获不处理|IGN|start default.target|Y|
|35) SIGRTMIN+1|捕获不处理|IGN|isolate rescue.target|Y|
|36) SIGRTMIN+2|捕获不处理|IGN|isolate emergency.target|Y|
|37) SIGRTMIN+3|捕获不处理|IGN|start halt.target|Y|
|38) SIGRTMIN+4|捕获不处理|IGN|start poweroff.target|Y|
|39) SIGRTMIN+5|捕获不处理|IGN|start reboot.target|Y|
|40) SIGRTMIN+6|捕获不处理|IGN|start kexec.target|Y|
|41) SIGRTMIN+7|捕获不处理|IGN|DFL|N|
|42) SIGRTMIN+8|unrecover state|IGN|DFL|N|
|43) SIGRTMIN+9|重执行sysmaster|IGN|DFL|N|
|44) SIGRTMIN+10|switch root|IGN|DFL|Y|
|45) SIGRTMIN+11|捕获不处理|IGN|DFL|N|
|46) SIGRTMIN+12|捕获不处理|IGN|DFL|N|
|47) SIGRTMIN+13|捕获不处理|IGN|Immediate halt|Y|
|48) SIGRTMIN+14|捕获不处理|IGN|Immediate poweroff|Y|
|49) SIGRTMIN+15|捕获不处理|IGN|Immediate reboot|Y|
|50) SIGRTMAX-14 SIGRTMIN+16|捕获不处理|IGN|Immediate kexec|Y|
|51) SIGRTMAX-13 SIGRTMIN+17|捕获不处理|IGN|DFL|N|
|52) SIGRTMAX-12 SIGRTMIN+18|捕获不处理|IGN|DFL|N|
|53) SIGRTMAX-11 SIGRTMIN+19|捕获不处理|IGN|DFL|N|
|54) SIGRTMAX-10 SIGRTMIN+20|捕获不处理|IGN|enable status messages|Y|
|55) SIGRTMAX-9 SIGRTMIN+21|捕获不处理|IGN|disable status messages|Y|
|56) SIGRTMAX-8 SIGRTMIN+22|捕获不处理|IGN|日志级别设为debug|Y|
|57) SIGRTMAX-7 SIGRTMIN+23|捕获不处理|IGN|日志级别设为info|Y|
|58) SIGRTMAX-6 SIGRTMIN+24|捕获不处理|IGN|Immediate exit (仅限于用户模式)|Y|
|59) SIGRTMAX-5 SIGRTMIN+25|捕获不处理|IGN|reexecute manager|Y|
|60) SIGRTMAX-4 SIGRTMIN+26|捕获不处理|IGN|日志输出设为journal-or-kmsg|Y|
|61) SIGRTMAX-3 SIGRTMIN+27|捕获不处理|IGN|日志输出设为console|Y|
|62) SIGRTMAX-2 SIGRTMIN+28|捕获不处理|IGN|日志输出设为kmsg|Y|
|63) SIGRTMAX-1 SIGRTMIN+29|捕获不处理|IGN|日志输出设为syslog-or-kmsg|Y|
|64) SIGRTMAX SIGRTMIN+30|捕获不处理|IGN|DFL|N|

表格的具体说明：

1. IGN、DFL分别表示信号处理函数：SIG_IGN（忽略）、SIG_DFL（缺省的信号处理函数）。如果1号进程没有注册信号处理函数，即使用SIG_DFL，内核会直接屏蔽掉该信号。因此1号进程在对外表现上，SIG_IGN和SIG_DFL是一致的。
2. SIGKILL、SIGSTOP信号是内核为1号进程无条件屏蔽的，且不允许通过sigaction修改其信号处理函数，sysmaster当前没有方案消除该差异。
3. init进程：
    - SIGCHLD: 回收所有僵尸子进程。
    - SIGRTMIN+8: 进入不可恢复状态。
    - SIGRTMIN+9: 下发SIGABRT信号重执行sysmaster。
    - SIGRTMIN+10: 只处理sysmaster进程的SIGRTMIN+10信号，其余进程忽略。重执行init自身，重执行sysmaster。
    - init捕获所有可捕获信号，除上述信号做了处理以外，其余都忽略不处理。
4. sysmaster进程：
    - SIGCHLD: 回收所有僵尸子进程。
    - SIGSEGV,SIGILL,SIGFPE,SIGBUS,SIGQUIT,SIGSYS: 当存在switch.debug配置文件时，执行故障恢复。
    - SIGABRT：当存在switch.debug配置文件时，用于init重执行sysmaster或执行故障恢复。
    - SIGUSR1、SIGUSR2、SIGWINCH、SIGPWR、SIGRTMIN+{0-6、13-16、20-29}sysmaster与systemd的差异后续能够消除。
