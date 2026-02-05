#!/bin/bash

# sudo addgroup dialout
# sudo usermod -a -G dialout $USER
#
# GID 100 or 1000 depending on distro
# The others are reserved
#
# cp ./99-ttyusb.rules /etc/udev/rules.d

sudo udevadm control --reload-rules
sudo udevadm trigger
