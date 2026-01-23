#!/bin/bash

sudo stty -F /dev/ttyUSB0 raw 9600 -echo -ixon -ixoff

while true; do
    echo -ne "R\r" > /dev/ttyUSB0
    timeout 1 cat /dev/ttyUSB0
    sleep 1
done
