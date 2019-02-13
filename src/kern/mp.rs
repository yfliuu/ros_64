use crate::*;
use core::mem::size_of;

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
// 3) in the BIOS ROM between 0xe0000 and 0xfffff.
#[allow(exceeding_bitshifts)]
fn mp_search() -> *const MP {
    fn mp_search_1(a: u64, len: u64) -> *const MP {
        let addr: u64 = p2v!(a);
        let e: u64 = addr + len;
        let mut p: u64 = addr;
        while p < e {
            if memcmp(p as *const u8, "_MP_".as_ptr(), 4) &&
                sum(p as *const u8, size_of::<MP>()) == 0 {
                return p as *const MP;
            }
            p = p + size_of::<MP>() as u64;
        }
        return 0x0 as *const MP;
    }

    let bda = p2v!(0x400) as *mut u8;
    unsafe {
        let p: u64 = (((*bda.offset(0x0f) as u64) << 8) |
            (*bda.offset(0x0e) as u64)) << 4;
        if p != 0x0 {
            // In QEMU we'll find it here
            let mp = mp_search_1(p as u64, 1024);
            if mp != 0x0 as *const MP { return mp; }
        }
        else {
            let p = (((*bda.offset(0x14) as u64) << 8) |
                ((*bda.offset(0x13) as u64) << 8)) << 10;
            let mp = mp_search_1(p as u64 - 1024, 1024);
            if mp != 0x0 as *const MP { return mp; }
        }
    }
    mp_search_1(0xf0000, 0x10000)
}

fn mp_config(pmp: *mut *const MP) -> *const MPConf {
    let mp = mp_search();
    unsafe {
        if mp == 0x0 as *const MP || (*mp).physaddr == 0 {
            return 0x0 as *const MPConf;
        }
        let phys_addr = (*mp).physaddr as u64;
        let conf = p2v!(phys_addr) as *const MPConf;
        if !memcmp(conf as *const u8, "PCMP".as_ptr(), 4) {
            return 0x0 as *const MPConf;
        }
        if (*conf).version != 1 && (*conf).version != 4 {
            return 0x0 as *const MPConf;
        }
        if sum(conf as *const u8, (*conf).length as usize) != 0 {
            return 0x0 as *const MPConf;
        }
        *pmp = mp;
        conf
    }
}

pub fn mpinit() -> () {
    let mut mp: *const MP = 0x0 as *const MP;
    let conf = mp_config(&mut mp as *mut *const MP);
    if conf == 0x0 as *const MPConf {
        panic!("Expect to run on an SMP");
    }
}