# sysmaster的信号处理

sysMaster采用`1+1+N`架构，sysmaster不再作为init进程，因此内核为init进程的信号做的特殊处理不再生效。下表列出了sysMaster与systemd针对不同信号的响应逻辑差异。

|信号|sysmaster|systemd|对外表现是否有差异|
|-|-|-|-|
|1) SIGHUP|daemon-reexec|daemon-reload|N|
|2) SIGINT|start ctrl-alt-del.target|start ctrl-alt-del.target|N|
|3) SIGQUIT|crash handler|crash handler|N|
|4) SIGILL|crash handler|crash handler|N|
|5) SIGTRAP|IGN|DFL|N|
|6) SIGABRT|crash handler|crash handler|N|
|7) SIGBUS|crash handler|crash handler|N|
|8) SIGFPE|crash handler|crash handler|N|
|9) SIGKILL|DFL|内核主动屏蔽该信号|Y|
|10) SIGUSR1|IGN|重连dbus|Y|
|11) SIGSEGV|crash handler|crash handler|N|
|12) SIGUSR2|IGN|输出所有单元的配置信息|Y|
|13) SIGPIPE|IGN|IGN|N|
|14) SIGALRM|IGN|DFL|N|
|15) SIGTERM|daemon-reexec|daemon-reexec|N|
|16) SIGSTKFLT|IGN|DFL|N|
|17) SIGCHLD|子进程回收|子进程回收|N|
|18) SIGCONT|IGN|DFL|N|
|19) SIGSTOP|DFL|内核主动屏蔽该信号|Y|
|20) SIGTSTP|IGN|DFL|N|
|21) SIGTTIN|IGN|DFL|N|
|22) SIGTTOU|IGN|DFL|N|
|23) SIGURG|IGN|DFL|N|
|24) SIGXCPU|IGN|DFL|N|
|25) SIGXFSZ|IGN|DFL|N|
|26) SIGVTALRM|IGN|DFL|N|
|27) SIGPROF|IGN|DFL|N|
|28) SIGWINCH|IGN|start kbrequest.target|Y|
|29) SIGIO|IGN|DFL|N|
|30) SIGPWR|IGN|start sigpwr.target|Y|
|31) SIGSYS|IGN|DFL|N|
|34) SIGRTMIN|IGN|start default.target|Y|
|35) SIGRTMIN+1|IGN|isolate rescue.target|Y|
|36) SIGRTMIN+2|IGN|isolate emergency.target|Y|
|37) SIGRTMIN+3|IGN|start halt.target|Y|
|38) SIGRTMIN+4|IGN|start poweroff.target|Y|
|39) SIGRTMIN+5|IGN|start reboot.target|Y|
|40) SIGRTMIN+6|IGN|start kexec.target|Y|
|41) SIGRTMIN+7|daemon-reexec|DFL|Y|
|42) SIGRTMIN+8|IGN|DFL|N|
|43) SIGRTMIN+9|IGN|DFL|N|
|44) SIGRTMIN+10|switch root|DFL|Y|
|45) SIGRTMIN+11|IGN|DFL|N|
|46) SIGRTMIN+12|IGN|DFL|N|
|47) SIGRTMIN+13|IGN|Immediate halt|Y|
|48) SIGRTMIN+14|IGN|Immediate poweroff|Y|
|49) SIGRTMIN+15|IGN|Immediate reboot|Y|
|50) SIGRTMAX-14 SIGRTMIN+16|IGN|Immediate kexec|Y|
|51) SIGRTMAX-13 SIGRTMIN+17|IGN|DFL|N|
|52) SIGRTMAX-12 SIGRTMIN+18|IGN|DFL|N|
|53) SIGRTMAX-11 SIGRTMIN+19|IGN|DFL|N|
|54) SIGRTMAX-10 SIGRTMIN+20|IGN|enable status messages|Y|
|55) SIGRTMAX-9 SIGRTMIN+21|IGN|disable status messages|Y|
|56) SIGRTMAX-8 SIGRTMIN+22|IGN|日志级别设为debug|Y|
|57) SIGRTMAX-7 SIGRTMIN+23|IGN|日志级别设为info|Y|
|58) SIGRTMAX-6 SIGRTMIN+24|IGN|Immediate exit (仅限于用户模式)|Y|
|59) SIGRTMAX-5 SIGRTMIN+25|IGN|reexecute manager|Y|
|60) SIGRTMAX-4 SIGRTMIN+26|IGN|日志输出设为journal-or-kmsg|Y|
|61) SIGRTMAX-3 SIGRTMIN+27|IGN|日志输出设为console|Y|
|62) SIGRTMAX-2 SIGRTMIN+28|IGN|日志输出设为kmsg|Y|
|63) SIGRTMAX-1 SIGRTMIN+29|IGN|日志输出设为syslog-or-kmsg|Y|
|64) SIGRTMAX SIGRTMIN+30|IGN|DFL|N|

表格的具体说明：

1. IGN、DFL分别表示信号处理函数：SIG_IGN（忽略）、SIG_DFL（缺省的信号处理函数）。如果init进程没有注册信号处理函数，即使用SIG_DFL，内核会直接屏蔽掉该信号。因此在对外表现上，SIG_IGN和SIG_DFL是一致的。
2. SIGKILL、SIGSTOP信号是内核为init进程无条件屏蔽的，且不允许通过sigaction修改其信号处理函数，sysmaster当前没有方案消除该差异。
3. SIGUSR1、SIGUSR2、SIGWINCH、SIGPWR、SIGRTMIN+{0-6、13-16、20-29}的差异后续能够消除。
4. SIGRTMIN+7在sysMaster中为init进程发给sysmaster进程，主动触发热重启。SIGRTMIN+10在sysMaster中为init进程发给sysmaster，执行switch root。
