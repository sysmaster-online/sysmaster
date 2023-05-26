#!/bin/bash
/usr/sbin/ifconfig eth0 0.0.0.0 up
eth0 x.xx.xx.xx/xx
/usr/sbin/route add default gw xx.xx.xx.xx
