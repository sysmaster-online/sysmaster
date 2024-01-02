#!/usr/bin/env bash
## PARAMS 1:input tarball name, for example:sysmasterwithbusybox.aarch64-1.0.tar.xz
TARPATH=$1
## PARAMS 2:specify turing version, for example:V1, V2
VERSION=$2
## PARAMS 3:output name
OUTNAME=$3

ROOTFSTMP=
TIMESTAMP="`date +'%F-%H-%M-%S'`"
FILENAME=`basename $0`
CURDIR=`pwd`
VM="sysmasterwithbusybox"

IP=9.43.254.249
NETMASK=255.255.0.0
GATEWAY=9.43.0.1

function fn_exit()
{
	exit $1
}

function fn_log()
{
	echo "[`date +'%F %T'`][$1]$2"
}

function fn_logerr()
{
	fn_log "ERROR" "$1"
	fn_exit 1
}

function fn_loginfo()
{
        fn_log "INFO" "$1"
}

function fn_logwarn()
{
        fn_log "WARN" "$1"
}

function fn_preparation()
{
	#01.verify TARPATH
	if [ -z ${TARPATH} ];then
		fn_logerr "please specify tarball path!"
	fi

	if [ ! -f ${TARPATH} ];then
		fn_logerr "file input ${TARPATH} no exist!"
	fi

	#02.verify VERSION
	if [ -z ${VERSION} ];then
		VERSION=debug
		fn_logwarn "  VERSION:${VERSION}(default)"
	else
		fn_loginfo "  VERSION:${VERSION}"
	fi

	#03.verify OUTNAME
	if [ -z ${OUTNAME} ];then
		OUTNAME=sysmasterwithbusybox-${VERSION}-${TIMESTAMP}.cpio.gz
		fn_logwarn "  OUTNAME:${OUTNAME}(default)"
	else
		fn_loginfo "  OUTNAME:${OUTNAME}"
	fi

	ROOTFSTMP=rootfs-tmp-${VERSION}-${TIMESTAMP}
	fn_loginfo "ROOTFSTMP:${ROOTFSTMP}"
}

function fn_remake()
{
	cd tmpdir||fn_logerr "failed to enter tmpdir!"
	mkdir ${ROOTFSTMP}
	cd ${ROOTFSTMP}
	tar -xf ../../${TARPATH}||fn_logerr "failed to unzip ../../${TARPATH}!"
	cp ../../extra/rcS etc/init.d/rcS

	#prepare for sshd
	echo "modprobe virtio_pci
	modprobe virtio_net
	ifconfig eth0 ${IP} netmask ${NETMASK}
	route add default gw ${GATEWAY}" >> etc/init.d/rcS

	cat ../../extra/start_sshd >> etc/init.d/rcS

	#move sysmaster from "extra" dir to the image
	cp -r ../../extra/usr/lib/* usr/lib/
	cp -r ../../extra/usr/bin/sctl usr/bin/
	mkdir -p etc/sysmaster/
	cp -r ../../extra/etc/sysmaster/* etc/sysmaster/

	sed -i 's/^PermitRootLogin.*/PermitRootLogin yes/g' etc/ssh/sshd_config

	#change sysmaster as init
	rm -rf sbin/init
	ln -s ../lib/sysmaster/sysmaster sbin/init
	cp boot/vmlinuz-*.*64 ${CURDIR}/Image.gz

	find . | cpio -H newc -o | pigz -9 -p 10 > ../../output/${OUTNAME}||fn_logerr "failed to generate filesystem!"
}

function fn_restart_vm()
{
	virsh list | grep -q ${VM}
	if [ $? -eq 0 ];then
		fn_loginfo "virsh destroy ${VM}"
		virsh destroy ${VM}
	fi

	virsh list --all | grep -q ${VM}
	if [ $? -eq 0 ];then
		fn_loginfo "virsh undefine ${VM}"
		virsh undefine ${VM}
	fi

	cp ${CURDIR}/extra/${VM}.xml . ||fn_logerr "failed to cp ${VM}.xml"
	sed -i "s/FSNAME/${OUTNAME}/g" ${VM}.xml

	virsh define ${VM}.xml

	fn_loginfo "virsh start ${VM}"
	virsh start ${VM}

        rm -f ${VM}.xml
	fn_loginfo "virsh console ${VM}"
        virsh console ${VM}
}

function fn_main()
{
	fn_loginfo "   CURDIR:${CURDIR}"
	rm -f ${CURDIR}/Image.gz
	rm -rf ${CURDIR}/tmpdir
	rm -rf ${CURDIR}/output
	mkdir -p ${CURDIR}/tmpdir
	mkdir -p ${CURDIR}/output
	fn_preparation
	fn_remake
	fn_restart_vm
}

fn_main
exit 0
