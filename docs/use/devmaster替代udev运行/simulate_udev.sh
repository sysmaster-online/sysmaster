#!/bin/bash

mv /run/udev /run/udev_back
mkdir -p /run/devmaster
cp -r /run/udev_back/data /run/devmaster/
ln -sf /run/devmaster /run/udev
