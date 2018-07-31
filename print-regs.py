#!/usr/bin/env python3

import sys

REGS = {
     'gpiob': ['moder', 'otyper', 'ospeedr', 'pupdr',
               'idr', 'odr', 'bsrr', 'lckr', 'afrl', 'afrh', 'brr'],
     'tim4': ['cr1', 'cr2', 'smcr', 'dier',
              'sr', 'egr', 'ccmr1_output', 'ccmr2_output', 'ccer',
              'cnt', 'psc', 'arr', 'rsrv', 'ccr1', 'ccr2', 'ccr3', 'ccr4', 'dcr']
}

def sreg(fn):
     z = open(fn,  "rb").read()
     num_regs = len(z) // 4
     for i in range(num_regs):
         buf = z[i*4:i*4+4]
         s = ''.join(bin(b)[2:].rjust(8, '0') for b in buf)
         yield s

r = sys.argv[1]
fn = sys.argv[2]
for (state, reg) in zip(sreg(fn), REGS.get(r)):
    print('{}: {}'.format(reg.rjust(12, ' '), state))
