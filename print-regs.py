#!/usr/bin/env python3

import sys

REGS = ['moder', 'otyper', 'ospeedr', 'pupdr',
        'idr', 'odr', 'bsrr', 'lckr', 'afrl', 'afrh', 'brr']

def sreg(fn):
     z = open(fn,  "rb").read()
     num_regs = len(z) // 4
     for i in range(num_regs):
         buf = z[i*4:i*4+4]
         s = ''.join(bin(b)[2:].rjust(8, '0') for b in buf)
         yield s

fn = sys.argv[1]
for (state, reg) in zip(sreg(fn), REGS):
    print('{}: {}'.format(reg.rjust(12, ' '), state))
