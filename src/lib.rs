#![no_std]
use lazy_static::lazy_static;
use x86_64::VirtAddr as VA;

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

pub const PGSIZE: u64 = 4096;