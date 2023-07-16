#!/usr/bin/env python3
import serial
import sys
import time
from serial.tools import list_ports

device = sys.argv[1]
if not device.startswith("/"):
    x = list(list_ports.grep(device))
    if not x:
        raise ArgumentError(f"Device {devici} not found")
    device = x[0].device
port = serial.Serial(device, baudrate=460800)

i = 0
while True:
    try:
        line = port.read_until()
        i = 0
        if line.strip():
            print(line.strip().decode('ascii', 'ignore'))
            sys.stdout.flush()
        else:
            print("fail")
    except serial.serialutil.SerialException as e:
        print(f"exception: {e}; sleeping for 1s or until device ({device}) is there...")
        time.sleep(1)
        while not list(list_ports.grep(device)):
            time.sleep(1)
        port = serial.Serial(device, baudrate=460800)
