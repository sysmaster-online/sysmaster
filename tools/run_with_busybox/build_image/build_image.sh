#!/usr/bin/env bash

WORKDIR="/tmp/image"
REPO=http://repo.openeuler.org/openEuler-22.09/everything/aarch64/

#backup /etc/yum.repos.d
mv /etc/yum.repos.d /etc/bak_yum.repos.d
yum clean all
mkdir -p /etc/yum.repos.d

echo "[openEuler22.09]
name=openEuler22.09
baseurl=${REPO}
enabled=1
gpgcheck=0" > /etc/yum.repos.d/openEuler22.09.repo

yum install -y kiwi-tools kiwi-cli python3-devel
pip install --upgrade kiwi

#clean WORKDIR
rm -rf ${WORKDIR}

kiwi-ng \
--debug system build \
--description kiwi \
--set-repo ${REPO} \
--target-dir ${WORKDIR}

#restore /etc/yum.repos.d
rm -rf /etc/yum.repos.d
mv /etc/bak_yum.repos.d /etc/yum.repos.d
