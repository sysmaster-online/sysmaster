MAX=10
# support signals:
# SIGQUIT SIGILL SIGABRT SIGBUS SIGFPE SIGSEGV SIGSYS
signals=(3 4 6 7 8 11 31)
signal_num="$(echo ${signals[*]} | wc -w)"

## usage: run sysmaster as daemon
function run_sysmaster() {
    /usr/lib/sysmaster/sysmaster &> /opt/sysmaster.log &
    sysmaster_pid=$!
    ps aux | grep -v grep | grep sysmaster | grep -w "${sysmaster_pid}" && return 0
    cat /opt/sysmaster.log
    return 1
}

## usage: background unit operation
function stress() {
    while ((1)); do
        sleep $((RANDOM % 3 + 1))
        date
        sctl start test2.service

        sleep $((RANDOM % 3 + 1))
        date
        sctl stop test2.service
    done
}

## usage: check unit function
function check_fun() {
    sctl start test1.service || return 1
    sleep 1
    ps -elf | grep -v grep | grep 'sleep 888' || return 1

    sctl stop test1.service || return 1
    sleep 1
    ps -elf | grep -v grep | grep 'sleep 888' && return 1
    return 0
}

## usage: background random kill
function random_kill() {
    local flag=0
    local signal_index="$((RANDOM % signal_num))"

    # random kill with random signal
    eval kill -\${signals[${signal_index}]} "${sysmaster_pid}"

    # check sysmaster
    sleep 1
    if [[ "$(ps aux | grep -v grep | grep sysmaster | wc -l)" -eq 1 ]]; then
        if ! ps -elf | grep -v grep | grep sysmaster | awk '{print $4}' | grep -wq "${sysmaster_pid}"; then
            echo "sysmaster pid changed!"
	    flag=1
        fi
    else
        echo "check the number of sysmaster process failed!"
        flag=1
    fi
    # debug
    ps -elf | grep -v grep | grep sysmaster

    # check log
    if [[ "$(grep 'INFO  sysmaster sysmaster running in system mode' /opt/sysmaster.log | wc -l)" -ne 2 ]]; then
        echo "check sysmaster.log failed!"
	flag=1
    fi
    cat /opt/sysmaster.log
    echo > /opt/sysmaster.log

    # check function
    check_fun || flag=1

    return "${flag}"
}

## usage: clean process
function clean() {
    local pid
    ps -elf | grep -v grep | grep sysmaster
    pid=$(ps -elf | grep -v grep | grep sysmaster | awk '{print $4}')
    [ -n "${pid}" ] && kill -9 ${pid}
    cat /opt/sysmaster.log
    [ -n "${stress_pid}" ] && kill -9 "${stress_pid}"
    cat /opt/stress.log
    rm -rf /opt/sysmaster.log /opt/stress.log
}

cp -arf /opt/test1.service /opt/test2.service /usr/lib/sysmaster || exit 1
mkdir -p /run/sysmaster/reliability
touch /run/sysmaster/reliability/switch.debug || exit 1
run_sysmaster || exit 1

stress &> /opt/stress.log &
stress_pid=$!

for ((i = 0; i < ${MAX}; ++i)); do
    sleep "$((RANDOM % 5 + 1))"
    random_kill && continue
    clean
    exit 1
done

clean
exit 0
