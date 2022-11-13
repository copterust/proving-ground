#!/usr/bin/env python3
import serial
import sys
import time

port = serial.Serial(sys.argv[1], baudrate=460800)

while True:
    line = port.read_until()
    if line.strip():
        print(line.strip().decode('ascii', 'ignore'))
        sys.stdout.flush()
    else:
        print("fail")
