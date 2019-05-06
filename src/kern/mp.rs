use crate::*;
use crate::kern::proc::ProcState;
use x86_64::structures::tss::TaskStateSegment;
use kern::proc::Context;
use kern::proc::Proc;



#[repr(C)]
pub struct MPProc {
    etype: u8,
    apic_id: u8,
    version: u8,
    flags: u8,
    signature: [u8; 4],
    feature: u32,
    reserved: [u8; 8],
}

#[repr(C)]
pub struct MPioapic {
    etype: u8,
    apic_no: u8,
    version: u8,
    flags: u8,
    addr: u64
}

#[repr(C)]
#[thread_local]
pub struct CPU {
    pub id: u8,
    pub apic_id: u8,
    pub scheduler: Context,
    pub taskstate: TaskStateSegment,
    pub gdt: Option<&'static [u64; 8]>,
    pub started: bool,
    pub ncli: u64,
    pub intena: bool,
    pub proc: Option<&'static mut Proc<'static>>,
}

static mut MY_CPU: CPU = CPU::new(0, 0);

impl CPU {
    const fn new(id: u8, apic_id: u8) -> Self {
        CPU {
            id: id,
            apic_id: apic_id,
            scheduler: Context::new(),
            taskstate: TaskStateSegment::new(),
            // GDTs may be per core
            gdt: None,
            started: false,
            ncli: 0,
            intena: false,
            proc: None,
        }
    }

    pub fn set_proc(&mut self, proc: &'static mut Proc) -> () {
        self.proc = Some(proc)
    }

    pub fn get_proc(&self) -> &Proc {
        if let Some(ref x) = self.proc {
            x
        } else { panic!("Proc empty!"); }
    }

//     pub fn get_mut_proc(&mut self) -> &mut Proc {
//         if let Some(ref mut p) = self.proc {
//             p
//         } else { panic!("Proc empty"); }
//     }

    pub fn set_proc_state(&mut self, state: ProcState) -> () {
        if let Some(ref mut p) = self.proc {
            p.set_state(state);
        } else { panic!("CPU has no proc!"); }
    }

    pub fn set_proc_chan(&mut self, chan: VA) -> () {
        if let Some(ref mut p) = self.proc {
            p.set_chan(chan);
        } else { panic!("CPU has no proc!"); }
    }

    pub fn clear_proc(&mut self) -> () { self.proc = None; }
}

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

impl MP {
    // Search for the MP Floating Pointer Structure, which according to the
    // spec is in one of the following three locations:
    // 1) in the first KB of the EBDA;
    // 2) in the last KB of system base memory;
    // 3) in the BIOS ROM between 0xe0000 and 0xfffff. (In QEMU we will find it in this option)
    unsafe fn search() -> Option<&'static MP> {
        unsafe fn mp_search_1(a: u64, len: u64) -> Option<&'static MP> {
            let addr: u64 = p2v!(a);
            let e = addr + len;
            let mut p = addr;
            while p < e {
                if memcmp(p as *const u8, "_MP_".as_ptr(), 4) &&
                    sum(p as *const u8, size_of::<MP>()) == 0 {
                    return Some(&*(p as *const MP));
                }
                p += size_of::<MP>() as u64;
            }
            None
        }

        let bda = p2v!(0x400) as *mut u8;
        let p: u64 = (((*bda.offset(0x0f) as u64) << 8) |
            (*bda.offset(0x0e) as u64)) << 4;

        if p == 0 {
            let p: u64 = (((*bda.offset(0x14) as u64) << 8) |
                ((*bda.offset(0x13) as u64) << 8)) << 10;
            mp_search_1(p - 1024, 1024)
        }
        else {
            mp_search_1(p, 1024).or_else(||{
                mp_search_1(0xf0000, 0x10000)
            })
        }
    }
}

#[repr(C)]
pub struct MPConf {
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

impl MPConf {
    fn is_valid(&self) -> bool { unsafe {
        memcmp(self.signature.as_ptr(), "PCMP".as_ptr(), 4) &&
            (self.version == 1 || self.version == 4) &&
            sum(self.signature.as_ptr(), self.length as usize) == 0
    } }

    unsafe fn config(option_mp: Option<&'static MP>) -> Option<&'static MPConf> {
        match option_mp {
            Some(mp) => {
                if mp.physaddr == 0 { return None }
                let phys_addr = mp.physaddr as u64;
                let conf = &*(p2v!(phys_addr) as *const MPConf);
                if !conf.is_valid() { None }
                else { Some(conf) }
            }
            None => None
        }
    }
}

// This struct is read only.
// Do not use the cpus array.
// TODO: clean up the cpus array. We use thread_local MY_CPU now.
pub struct CpuInfo {
    cpus: [(u8, u8); MAX_CPU],
    lapic: VA,
    ncpu: u8,
    ioapicid: u8,
}

impl CpuInfo {
    unsafe fn init() -> Self {
        let mp = MP::search();
        let opt_conf = MPConf::config(mp);
        let mut cpus: [(u8, u8); MAX_CPU] = [(0, 0); MAX_CPU];
        let mut ncpu: u8 = 0;
        let mut ioapic_id: u8 = 0;

        match opt_conf {
            Some(conf) => {
                let ptr = conf as *const MPConf;
                let mut p = ptr.offset(1) as *const u8;
                let length = conf.length;
                let e = (ptr as *const u8).offset(length as isize);
                println!("lapic pa: 0x{:x}", (*conf).lapicaddr);
                while p < e {
                    match *p {
                        MPPROC => {
                            let proc = p as *const MPProc;
                            cpus[ncpu as usize] = (ncpu, (*proc).apic_id);
                            MY_CPU.apic_id = (*proc).apic_id;
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
                    ncpu: ncpu,
                    ioapicid: ioapic_id,
                }
            }
            _ => panic!("Expect to run on an SMP"),
        }
    }

    pub fn get_lapic(&self) -> VA { self.lapic }
    pub fn ioapic_id(&self) -> u8 { self.ioapicid }
}

lazy_static! {
    pub static ref CPU_INFO: CpuInfo = unsafe { CpuInfo::init() };
}

pub fn mp_init() -> () {
    // Read the static variable to trigger init()
    for i in 0..CPU_INFO.ncpu {
        println!("cpu{}: id: {}, apic_id: {}", i, CPU_INFO.cpus[i as usize].0,
                 CPU_INFO.cpus[i as usize].1);
    }
    println!("lapic: 0x{:x}", CPU_INFO.lapic.as_u64());
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

// Return current cpu that calls this function
pub unsafe fn my_cpu() -> &'static mut CPU {
    &mut MY_CPU
}
