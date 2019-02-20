use crate::*;
use volatile::Volatile;
use spin::Mutex;


#[repr(C)]
struct Ioapic {
    reg: Volatile<u32>,
    pad: [u32; 3],
    data: Volatile<u32>,
}

lazy_static! {
    static ref IOAPIC: Mutex<&'static mut Ioapic> = unsafe { Mutex::new(&mut *(io2v!(IOAPIC_ADDR) as *mut Ioapic)) };
}

impl Ioapic {
    fn init(&mut self) -> () {
        let maxintr = (self.read(REG_VER) >> 16) & 0xFF;
        let id = self.read(REG_ID) >> 24;
        if id != kern::mp::CPU_INFO.ioapic_id() as u32 {
            panic!("ID do not match! Perhaps not MP");
        }

        // Mark all interrupts edge-triggered, active high, disabled,
        // and not routed to any CPUs.
        for i in 0..maxintr + 1 {
            self.write(REG_TABLE + 2 * i, INT_DISABLED | (T_IRQ0 + i));
            self.write(REG_TABLE + 2 * i + 1, 0);
        }
    }

    fn write(&mut self, reg: u32, data: u32) -> () {
        self.reg.write(reg);
        self.data.write(data);
    }

    fn read(&mut self, reg: u32) -> u32 {
        self.reg.write(reg);
        self.data.read()
    }

    fn enable(&mut self, irq: u32, cpunum: u32) -> () {
        // Mark interrupt edge-triggered, active high,
        // enabled, and routed to the given cpunum,
        // which happens to be that cpu's APIC ID.
        self.write(REG_TABLE + 2 * irq, T_IRQ0 + irq);
        self.write(REG_TABLE + 2 * irq + 1, cpunum << 24);
    }
}

pub fn ioapic_init() -> () {
    IOAPIC.lock().init();
}

pub fn ioapic_enable(irq: u32, cpunum: u32) -> () {
    IOAPIC.lock().enable(irq, cpunum);
}