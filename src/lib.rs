#![no_std]
#![feature(asm)]

use lazy_static::lazy_static;
use x86_64::VirtAddr as VA;
use core::ptr::write_bytes;
use core::mem::size_of;

pub mod kern;

// UART. The SerialPort::new is a const fn.
const COM1: u16 = 0x3f8;
pub const UART: uart_16550::SerialPort = uart_16550::SerialPort::new(COM1);

extern "C" {
    // When use the symbol defined in linker script,
    // use the ADDRESS of the variable, never use the value.
    // See https://sourceware.org/binutils/docs/ld/Source-Code-Reference.html
    static KERNEL_BASE: u64;
    static KERNEL_END: u64;
}

// Use these. See kernel.ld for more details.
lazy_static! {
    pub static ref KERN_BASE: VA = VA::from_ptr(unsafe {&KERNEL_BASE as *const u64});
    pub static ref KERN_END: VA = VA::from_ptr(unsafe {&KERNEL_END as *const u64});
}

// A simple wrapper
pub fn memset<T>(dst: *mut T, val: u8, count: u64) -> () {
    if count as usize % size_of::<T>()!= 0 { panic!("memset") }
    unsafe {
        write_bytes(dst, val, (count as usize) / size_of::<T>());
    }
}

pub fn memcmp(src: *const u8, dst: *const u8, len: u64) -> bool {
    unsafe {
        for i in 0..len as isize {
            if *src.offset(i) != *dst.offset(i) {
                return false
            }
        }
    }
    true
}

pub fn ptr2u64<T>(ptr: *mut T) -> u64 {
    use usize_conversions::FromUsize;

    u64::from_usize(ptr as usize)
}


// ----------MEM LAYOUT----------
pub const PGSIZE: u64 = 4096; // 4KB page
pub const ENTRY_COUNT: usize = 512; // Entries per page
pub const DEVBASE: u64 = 0xffffffff40000000; // first device virtual address
pub const DEVSPACE: u64 = 0xfe000000;
pub const PHYSTOP: u64 = 0x20000000; // 512MB memory



// ----------PAGE TABLE ENTRY BIT FLAGS---------
pub const PTE_P  : u64 = 0x001;   // Present
pub const PTE_W  : u64 = 0x002;   // Writeable
pub const PTE_U  : u64 = 0x004;   // User
pub const PTE_PWT: u64 = 0x008;   // Write-Through
pub const PTE_PCD: u64 = 0x010;   // Cache-Disable
pub const PTE_A  : u64 = 0x020;   // Accessed
pub const PTE_D  : u64 = 0x040;   // Dirty
pub const PTE_PS : u64 = 0x080;   // Page Size
pub const PTE_MBZ: u64 = 0x180;   // Bits must be zero