FROM openeuler-22.03-lts-sp1:latest
MAINTAINER <shenyangyang4@huawei.com>
RUN yum -y install openssh-clients

RUN mkdir -p /usr/lib/sysmaster/plugin
ADD ./debug/sctl /usr/bin
ADD ./debug/sysmaster /usr/lib/sysmaster
ADD ./debug/fstab /usr/lib/sysmaster
ADD ./debug/sysmonitor /usr/lib/sysmaster
ADD ./debug/random_seed /usr/lib/sysmaster
ADD ./debug/rc-local-generator /usr/lib/sysmaster
ADD ./debug/libmount.so /usr/lib/sysmaster/plugin
ADD ./debug/libservice.so /usr/lib/sysmaster/plugin
ADD ./debug/libsocket.so /usr/lib/sysmaster/plugin
ADD ./debug/libtarget.so /usr/lib/sysmaster/plugin
ADD ./debug/conf/plugin.conf /usr/lib/sysmaster/plugin

RUN rm -f /sbin/init
ADD ./debug/init /sbin/

ADD ./debug/basic.target /usr/lib/sysmaster
ADD ./debug/Multi-user.target /usr/lib/sysmaster
ADD ./debug/shutdown.target /usr/lib/sysmaster
ADD ./debug/sysinit.target /usr/lib/sysmaster
ADD sshd.service /usr/lib/sysmaster
ADD sshd-keygen.target /usr/lib/sysmaster
ADD sshd-keygen@rsa.service /usr/lib/sysmaster
ADD sshd-keygen@ecdsa.service /usr/lib/sysmaster
ADD sshd-keygen@ed25519.service /usr/lib/sysmaster

ENV TMOUT=0
CMD [/sbin/init]
