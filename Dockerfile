FROM scratch
COPY target/x86_64-unknown-linux-musl/debug/init /sbin/init
COPY target/x86_64-unknown-linux-musl/debug/process1 /usr/lib/process1/process1
#COPY target/x86_64-unknown-linux-musl/debug/systemd /usr/lib/systemd/systemd
CMD ["/sbin/init"]
