#!/usr/bin/env python3
import serial
import sys
import time

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3)

while True:
    i = input().strip()
    port.write(i.encode('ascii') + b'\n')
    line = port.read_until()
    if line.strip():
        print(line.strip().decode('ascii', 'ignore'))
    else:
        print("fail")
