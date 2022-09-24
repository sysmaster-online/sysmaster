#这只是其中的一部分
Name:           systemd
Url:            https://www.freedesktop.org/wiki/Software/systemd
Version:        248
Release:        13.test42
License:        MIT and LGPLv2+ and GPLv2+
Summary:        System and Service Manager

#add these patches in systemd.spec
Patch0101:	0001-fake-pid1-by-env.patch
Patch0102:	0002-detect-virt.patch
Patch0103:	0003-reap-child-by-systemd.patch
Patch0104:	0004-add-signal.patch
Patch0105:	0005-do-not-wait-in-shutdown.patch
Patch0106:	0006-reexec-when-crash.patch
