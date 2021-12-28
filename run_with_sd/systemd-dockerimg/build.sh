#!/bin/bash
yum -c openEuler.repo  --installroot=$PWD/rootfs install systemd -y
cp -a rpms/systemd*.rpm rootfs/
if [ $? -ne 0 ]; then
	echo "put your rpms into dir rpms"
	exit 1
fi
chroot rootfs  /bin/bash -c "rpm -Fvh *"
rm -rf rootfs/systemd*.rpm
docker build --no-cache --tag systemd .
