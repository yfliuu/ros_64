use kern::mp::CPU_INFO;
use crate::*;
use volatile::Volatile;
use x86_64::instructions::port::Port;

#[repr(transparent)]
pub struct Lapic {
    lapic: u64,
}

lazy_static! {
    static ref LAPIC: Lapic = Lapic::new();
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
        let ptr = self.lapic as *mut Volatile<u32>;
        unsafe {
            (*(ptr.offset(i as isize))).write(v);
            // Wait for write to finish, by reading
            (*(ptr.offset(ID as isize))).read();
        }
    }

    fn rd(&self, i: u32) -> u32 {
        let ptr = self.lapic as *mut Volatile<u32>;
        unsafe { (*(ptr).offset(i as isize)).read() }
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

    // Acknowledge interrupt.
    pub fn lapic_eoi(&self) -> () { self.wrt(EOI, 0) }
    pub unsafe fn lapic_id(&self) -> u64 {
        let ptr = self.lapic as *mut Volatile<u32>;
        (*(ptr.offset(ID as isize))).read() as u64
    }

    pub unsafe fn start_ap(&self, apic_id: u8, addr: VA) -> () {
        // "The BSP must initialize CMOS shutdown code to 0AH
        // and the warm reset vector (DWORD based at 40:67) to point at
        // the AP startup code prior to the [universal startup algorithm]."
        Port::new(CMOS_PORT as u16).write(0xF as u32);  // offset 0xF is shutdown code
        Port::new((CMOS_PORT + 1) as u16).write(0x0A as u32);
        let wrv = VA::new(p2v!((0x40<<4 | 0x67))).as_mut_ptr::<u16>(); // Warm reset vector

        *wrv.offset(0) = 0;
        *wrv.offset(1) = (addr.as_u64() >> 4) as u16;

        // "Universal startup algorithm."
        // Send INIT (level-triggered) interrupt to reset other CPU.
        self.wrt(ICRHI, (apic_id as u32) << 24);
        self.wrt(ICRLO, INIT | LEVEL | ASSERT);
        self.wrt(ICRLO, INIT | LEVEL);

        // Send startup IPI (twice!) to enter code.
        // Regular hardware is supposed to only accept a STARTUP
        // when it is in the halted state due to an INIT.  So the second
        // should be ignored, but it is part of the official Intel algorithm.
        // Bochs complains about the second one.  Too bad for Bochs.
        for _ in 0..2 {
            self.wrt(ICRHI, (apic_id as u32) << 24);
            self.wrt(ICRLO, STARTUP | (addr.as_u64() >> 12) as u32);
        }
    }
}

pub fn lapic_init() -> () {
    // Setting up LAPIC.
    LAPIC.init();

    // Disable the 8259A because we're in SMP environment.
    // OSDevWiki: Disable the 8259 PIC properly is nearly as important as setting up the APIC.
    const IO_PIC1: u16 = 0x20;
    const IO_PIC2: u16 = 0xA0;

    unsafe {
        Port::new(IO_PIC1 + 1 as u16).write(0xff as u8);
        Port::new(IO_PIC2 + 1 as u16).write(0xff as u8);
    }
}

pub fn lapic_eoi() -> () {
    LAPIC.lapic_eoi();
}

// Enable interrupt on this processor
pub unsafe fn sti() -> () {
    asm!("sti");
}

// Return calling processor's lapic_id.
pub unsafe fn lapic_id() -> u64 { LAPIC.lapic_id() }

pub unsafe fn lapic_start_ap(apic_id: u8, addr: VA) -> () {
    LAPIC.start_ap(apic_id, addr);
}
