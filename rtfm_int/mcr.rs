macro_rules! flash {
    ($p: expr, $d: expr) => {
        for _ in 1..10 {
            $p.bsrr.write(|w| w.bs5().set_bit());
            $d.delay_ms(100u32);
            $p.bsrr.write(|w| w.bs5().clear_bit());
            $d.delay_ms(100u32);
        }
    };
}
