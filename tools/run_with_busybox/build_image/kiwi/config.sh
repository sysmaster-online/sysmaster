#!/usr/bin/env bash

busybox_file=/usr/share/busybox/busybox.links

#remove the files we don't need and
#create symbolic links for busybox

rm -rf /etc/systemd
rm -rf /etc/udev
rm -rf /usr/lib/systemd
rm -rf /usr/lib/udev
rm -rf /usr/sbin/init

for cmd in `cat ${busybox_file}`
do
	{
	        if [ -e $cmd ]
                then
                        rm -f $cmd
                fi

                ln -s /usr/sbin/busybox $cmd
        } &

done
