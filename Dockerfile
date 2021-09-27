
FROM scratch
#ADD target/x86_64-unknown-linux-musl/debug/process1 /sbin/init
COPY target/x86_64-unknown-linux-musl/debug/init /sbin/init
CMD ["/sbin/init"]
