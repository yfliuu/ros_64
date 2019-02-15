use crate::*;
use volatile::Volatile;
use core::mem::uninitialized;

#[repr(C)]
pub struct MP {
    signature: [u8; 4],
    physaddr: u32,
    length: u8,
    specrev: u8,
    checksum: u8,
    mtype: u8,
    imcrp: u8,
    reserved: [u8; 3]
}

#[repr(C)]
struct MPConf {
    signature: [u8; 4],
    length: u16,
    version: u8,
    checksum: u8,
    product: [u8; 20],
    oemtable: u32,
    oemlength: u16,
    entry: u16,
    lapicaddr: u32,
    xlength: u16,
    xchecksum: u8,
    reserved: u8,
}

#[repr(C)]
struct MPProc {
    etype: u8,
    apic_id: u8,
    version: u8,
    flags: u8,
    signature: [u8; 4],
    feature: u32,
    reserved: [u8; 8],
}

#[repr(C)]
struct MPioapic {
    etype: u8,
    apic_no: u8,
    version: u8,
    flags: u8,
    addr: u64
}

#[repr(C)]
struct CPU {
    id: u8,
    apic_id: u8,
    // scheduler
    // taskstate
    // segdesc
    started: Volatile<bool>,
    ncli: u64,
    intena: bool
}

pub struct CpuInfo {
    cpus: [CPU; MAX_CPU],
    lapic: VA,
    is_mp: bool,
    ncpu: u8,
    ioapicid: u8,
}

impl MP {
    // Search for the MP Floating Pointer Structure, which according to the
    // spec is in one of the following three locations:
    // 1) in the first KB of the EBDA;
    // 2) in the last KB of system base memory;
    // 3) in the BIOS ROM between 0xe0000 and 0xfffff. (In QEMU we will find it in this option)
    fn search() -> Option<*const MP> {
        fn mp_search_1(a: u64, len: u64) -> Option<*const MP> {
            let addr: u64 = p2v!(a);
            let e: u64 = addr + len;
            let mut p: u64 = addr;
            while p < e {
                if memcmp(p as *const u8, "_MP_".as_ptr(), 4) &&
                    sum(p as *const u8, size_of::<MP>()) == 0 {
                    return Some(p as *const MP);
                }
                p = p + size_of::<MP>() as u64;
            }
            None
        }

        let bda = p2v!(0x400) as *mut u8;
        unsafe {
            let p: u64 = (((*bda.offset(0x0f) as u64) << 8) |
                (*bda.offset(0x0e) as u64)) << 4;

            match p {
                0 => {
                    let p = (((*bda.offset(0x14) as u64) << 8) |
                        ((*bda.offset(0x13) as u64) << 8)) << 10;
                    mp_search_1(p as u64 - 1024, 1024)
                }
                _ => { match mp_search_1(p as u64, 1024) {
                    Some(x) => Some(x),
                    None => mp_search_1(0xf0000, 0x10000)
                } }
            }
        }
    }
}

impl MPConf {
    fn is_valid(&self) -> bool {
        memcmp(self.signature.as_ptr(), "PCMP".as_ptr(), 4) &&
            (self.version == 1 || self.version == 4) &&
            sum(self.signature.as_ptr(), self.length as usize) == 0
    }

    fn config(option_mp: Option<*const MP>) -> Option<*const MPConf> {
        match option_mp {
            Some(mp) => { unsafe {
                if (*mp).physaddr == 0 { return None }
                let phys_addr = (*mp).physaddr as u64;
                let conf = p2v!(phys_addr) as *const MPConf;
                if !(*conf).is_valid() { return None }
                Some(conf)
            } }
            None => None
        }
    }
}

impl CPU {
    fn new(id: u8, apic_id: u8) -> Self {
        CPU {
            id: id,
            apic_id: apic_id,
            started: Volatile::new(false),
            ncli: 0,
            intena: false,
        }
    }
}

impl CpuInfo {
    fn init() -> Self {
        let mp = MP::search();
        let opt_conf = MPConf::config(mp);
        let mut cpus: [CPU; MAX_CPU] = unsafe { uninitialized() };
        let mut ncpu: u8 = 0;
        let mut ioapic_id: u8 = 0;

        match opt_conf {
            Some(conf) => { unsafe {
                let mut p = conf.offset(1) as *const u8;
                let length = (*conf).length;
                let e = (conf as *const u8).offset(length as isize);
                while p < e {
                    match *p {
                        MPPROC => {
                            let proc = p as *const MPProc;
                            cpus[ncpu as usize] = CPU::new(ncpu, (*proc).apic_id);
                            ncpu += 1;
                            p = p.offset(size_of::<MPProc>() as isize);
                        }
                        MPIOAPIC => {
                            ioapic_id = (*(p as *const MPioapic)).apic_no;
                            p = p.offset(size_of::<MPioapic>() as isize);
                        }
                        MPBUS |
                        MPIOINTR |
                        MPLINTR => p = p.offset(8),
                        x => println!("mpinit: unknown config type {:x}", x),
                    }
                }
                CpuInfo {
                    cpus: cpus,
                    lapic: VA::new(io2v!((*conf).lapicaddr as u64)),
                    is_mp: true,
                    ncpu: ncpu,
                    ioapicid: ioapic_id,
                }
            } }
            _ => panic!("Expect to run on an SMP"),
        }
    }

    pub fn get_lapic(&self) -> VA { self.lapic }
}

lazy_static! {
    pub static ref CPU_INFO: CpuInfo = CpuInfo::init();
}

pub fn mp_init() -> () {
    // Read the static variable to trigger init()
    for i in 0..CPU_INFO.ncpu {
        println!("cpu{}: id: {}, apic_id: {}", i, CPU_INFO.cpus[i as usize].id,
                 CPU_INFO.cpus[i as usize].apic_id);
    }
    println!("ncpu: {}", CPU_INFO.ncpu);
    println!("lapic: {:x}", CPU_INFO.lapic.as_u64());
    println!("ioapicid: {}", CPU_INFO.ioapicid);
}

// this sum is used to calculate checksum.
// the spec requires that all fields add up to 0
// and all fields are unsigned.
// But Rust will panic on overflow
// So we manually mod 256 to avoid overflow.
fn sum(a: *const u8, len: usize) -> u64 {
    let mut sum: u64 = 0;
    unsafe {
        for i in 0..len as isize {
            sum = (sum + *a.offset(i) as u64) % 256;
        }
    }
    sum
}