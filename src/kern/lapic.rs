use kern::mp::CPU_INFO;
use crate::*;


pub struct Lapic {
    lapic: u64,
}

lazy_static! {
    pub static ref LAPIC: Lapic = Lapic::new();
}

impl Lapic {
    fn new() -> Self {
        Lapic {
            lapic: match CPU_INFO.get_lapic().as_u64() {
                0 => panic!("LAPIC address null"),
                x => x,
            } }
    }

    fn wrt(&self, i: u32, v: u32) -> () {
        unsafe {
            *(self.lapic as *mut u32).offset(i as isize) = v;

            // Wait for write to finish, by reading
            let _ = *(self.lapic as *mut u32).offset(ID as isize);
        }
    }

    fn rd(&self, i: u32) -> u32 {
        unsafe { *(self.lapic as *mut u32).offset(i as isize) }
    }

    fn init(&self) -> () {
        // Enable local APIC; set spurious interrupt vector.
        self.wrt(SVR, ENABLE | (T_IRQ0 + IRQ_SPURIOUS));

        // The timer repeatedly counts down at bus frequency
        // from lapic[TICR] and then issues an interrupt.
        // If xv6 cared more about precise timekeeping,
        // TICR would be calibrated using an external time source.
        self.wrt(TDCR, X1 as u32);
        self.wrt(TIMER, PERIODIC | (T_IRQ0 + IRQ_TIMER));
        self.wrt(TICR, 10000000);

        // Disable logical interrupt lines.
        self.wrt(LINT0, MASKED);
        self.wrt(LINT1, MASKED);

        // Disable performance counter overflow interrupts
        // on machines that provide that interrupt entry.
        if ((self.rd(VER) >> 16) & 0xFF) >= 4 {
            self.wrt(PCINT, MASKED);
        }

        // Map error interrupt to IRQ_ERROR.
        self.wrt(ERROR, T_IRQ0 + IRQ_ERROR);

        // Clear error status register (requires back-to-back writes).
        self.wrt(ESR, 0);
        self.wrt(ESR, 0);

        // Ack any outstanding interrupts.
        self.wrt(EOI, 0);

        // Send an Init Level De-Assert to synchronise arbitration ID's.
        self.wrt(ICRHI, 0);
        self.wrt(ICRLO, BCAST | INIT | LEVEL);

        while self.rd(ICRLO) & DELIVS != 0 { }
        // Enable interrupts on the APIC (but not on the processor).
        self.wrt(TPR, 0);
    }
}

pub fn lapic_init() -> () {
    LAPIC.init();
}
