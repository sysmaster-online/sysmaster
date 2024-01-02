#!/usr/bin/env bash
yum -c openEuler.repo --repo=everything  --installroot=$PWD/rootfs install systemd -y
mkdir -p rootfs/rpms
cp -a rpms/systemd*.rpm rootfs/rpms
if [ $? -ne 0 ]; then
	echo "put your rpms into dir rpms"
	exit 1
fi
chroot rootfs /usr/bin/env bash -c "rpm -Fvh /rpms/*"
rm -rf rootfs/rpms/systemd*.rpm
docker build --no-cache --tag systemd .
