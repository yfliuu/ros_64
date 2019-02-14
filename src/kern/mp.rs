use crate::*;

#[repr(C)]
struct MP {
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

// Search for the MP Floating Pointer Structure, which according to the
// spec is in one of the following three locations:
// 1) in the first KB of the EBDA;
// 2) in the last KB of system base memory;
// 3) in the BIOS ROM between 0xe0000 and 0xfffff. (In QEMU we will find it in this option)
fn mp_search() -> Option<*const MP> {
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
            _ => {
                match mp_search_1(p as u64, 1024) {
                    Some(x) => Some(x),
                    None => mp_search_1(0xf0000, 0x10000)
                }
            }
        }
    }
}

fn mp_config(pmp: *mut *const MP) -> Option<*const MPConf> {
    // Check if the MPConf is valid
    fn check_conf(conf: *const MPConf) -> bool {
        unsafe {
            memcmp(conf as *const u8, "PCMP".as_ptr(), 4) &&
                ((*conf).version == 1 || (*conf).version == 4) &&
                sum(conf as *const u8, (*conf).length as usize) == 0
        }
    }

    let option_mp = mp_search();

    match option_mp {
        Some(mp) => {
            unsafe {
                if (*mp).physaddr == 0 { return None }
                let phys_addr = (*mp).physaddr as u64;
                let conf = p2v!(phys_addr) as *const MPConf;
                if !check_conf(conf) { return None }
                *pmp = mp;
                Some(conf)
            }
        }
        None => None
    }
}

// The lapic address will be returned
pub fn mpinit() -> u64 {
    let mut mp: *const MP = null_mut();
    let opt_conf = mp_config(&mut mp as *mut *const MP);
    match opt_conf {
        Some(conf) => {
            unsafe { (*conf).lapicaddr as u64 }
        }
        None => panic!("Expect to run on an SMP"),
    }
}