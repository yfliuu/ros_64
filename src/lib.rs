#![no_std]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(const_raw_ptr_deref)]
#![feature(ptr_internals)]
#![feature(allocator_api)]
#![feature(thread_local)]

use lazy_static::lazy_static;
use x86_64::VirtAddr as VA;
use x86_64::PhysAddr as PA;
use core::ptr::write_bytes;
use core::mem::size_of;
use core::option::Option;
use core::ptr::{null_mut};

pub mod kern;

extern "C" {
    // When use the symbol defined in linker script,
    // use the ADDRESS of the variable, never use the value.
    // See https://sourceware.org/binutils/docs/ld/Source-Code-Reference.html
    static _KERNEL_BASE: u64;
    static _KERNEL_END: u64;
    static _KERNEL_DATA: u64;
}

// Use these. See kernel.ld for more details.
lazy_static! {
    pub static ref KERN_BASE: VA = VA::from_ptr(unsafe {&_KERNEL_BASE as *const u64});
    pub static ref KERN_END: VA = VA::from_ptr(unsafe {&_KERNEL_END as *const u64});
    pub static ref KERN_DATA: VA = VA::from_ptr(unsafe {&_KERNEL_DATA as *const u64});
}


// A simple wrapper
pub unsafe fn memset<T>(dst: *mut T, val: u8, count:u64) -> () {
    if count as usize % size_of::<T>()!= 0 { panic!("memset") }
    write_bytes(dst, val, (count as usize) / size_of::<T>());
}

pub unsafe fn memcmp(src: *const u8, dst: *const u8, len: u64) -> bool {
    for i in 0..len as isize {
        if *src.offset(i) != *dst.offset(i) {
            return false
        }
    }
    true
}

pub unsafe fn memmove<T>(dst: *mut T, src: *mut T, len: usize) -> () {
    if len % size_of::<T>()!= 0 { panic!("memmove") }
    src.copy_to(dst, len / size_of::<T>());
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// ----------MEM LAYOUT----------
pub const PGSIZE:u64 = 4096; // 4KB page
pub const ENTRY_COUNT: usize = 512; // Entries per page
pub const DEVBASE: u64 = 0xffffffff40000000; // first device virtual address
pub const DEVSPACE: u64 = 0xfe000000;
pub const PHYSTOP: u64 = 0x20000000; // 512MB memory
pub const EXTMEM: u64 = 0x100000;

// -----------MP TABLE ENTRY--------------------
pub const MAX_CPU   : usize = 8;
pub const MPPROC    : u8 = 0x00;  // One per processor
pub const MPBUS     : u8 = 0x01;  // One per bus
pub const MPIOAPIC  : u8 = 0x02;  // One per I/O APIC
pub const MPIOINTR  : u8 = 0x03;  // One per bus interrupt source
pub const MPLINTR   : u8 = 0x04;  // One per system interrupt source

// --------------LAPIC REGISTERS----------------
// divided by 4 for use as indices
// Local APIC registers, divided by 4 for use as uint[] indices.
pub const ID       :u32 = (0x0020/4);   // ID
pub const VER      :u32 = (0x0030/4);   // Version
pub const TPR      :u32 = (0x0080/4);   // Task Priority
pub const EOI      :u32 = (0x00B0/4);   // EOI
pub const SVR      :u32 = (0x00F0/4);   // Spurious Interrupt Vector
pub const ENABLE   :u32 = 0x00000100;   // Unit Enable
pub const ESR      :u32 = (0x0280/4);   // Error Status
pub const ICRLO    :u32 = (0x0300/4);   // Interrupt Command
pub const INIT     :u32 = 0x00000500;   // INIT/RESET
pub const STARTUP  :u32 = 0x00000600;   // Startup IPI
pub const DELIVS   :u32 = 0x00001000;   // Delivery status
pub const ASSERT   :u32 = 0x00004000;   // Assert interrupt (vs deassert)
pub const DEASSERT :u32 = 0x00000000;
pub const LEVEL    :u32 = 0x00008000;   // Level triggered
pub const BCAST    :u32 = 0x00080000;   // Send to all APICs, including self.
pub const BUSY     :u32 = 0x00001000;
pub const FIXED    :u32 = 0x00000000;
pub const ICRHI    :u32 = (0x0310/4);   // Interrupt Command [63:32]
pub const TIMER    :u32 = (0x0320/4);   // Local Vector Table 0 (TIMER)
pub const X1       :u32 = 0x0000000B;   // divide counts by 1
pub const PERIODIC :u32 = 0x00020000;   // Periodic
pub const PCINT    :u32 = (0x0340/4);   // Performance Counter LVT
pub const LINT0    :u32 = (0x0350/4);   // Local Vector Table 1 (LINT0)
pub const LINT1    :u32 = (0x0360/4);   // Local Vector Table 2 (LINT1)
pub const ERROR    :u32 = (0x0370/4);   // Local Vector Table 3 (ERROR)
pub const MASKED   :u32 = 0x00010000;   // Interrupt masked
pub const TICR     :u32 = (0x0380/4);   // Timer Initial Count
pub const TCCR     :u32 = (0x0390/4);   // Timer Current Count
pub const TDCR     :u32 = (0x03E0/4);   // Timer Divide Configuration

// ----------------IOAPIC--------------------
pub const IOAPIC_ADDR:  u64 = 0xFEC00000;   // Default physical address of IO APIC

pub const REG_ID:       u32 = 0x00;  // Register index: ID
pub const REG_VER:      u32 = 0x01;  // Register index: version
pub const REG_TABLE:    u32 = 0x10;  // Redirection table base

// The redirection table starts at REG_TABLE and uses
// two registers to configure each interrupt.
// The first (low) register in a pair contains configuration bits.
// The second (high) register contains a bitmask telling which
// CPUs can serve that interrupt.
pub const INT_DISABLED:     u32 = 0x00010000;       // Interrupt disabled
pub const INT_LEVEL:        u32 = 0x00008000;       // Level-triggered (vs edge-)
pub const INT_ACTIVELOW:    u32 = 0x00002000;       // Active low (vs high)
pub const INT_LOGICAL:      u32 = 0x00000800;       // Destination is CPU id (vs APIC ID)

// ---------------TRAPS----------------------------
// Processor-defined:
pub const T_DIVIDE     :u32 =    0;      // divide error
pub const T_DEBUG      :u32 =    1;      // debug exception
pub const T_NMI        :u32 =    2;      // non-maskable interrupt
pub const T_BRKPT      :u32 =    3;      // breakpoint
pub const T_OFLOW      :u32 =    4;      // overflow
pub const T_BOUND      :u32 =    5;      // bounds check
pub const T_ILLOP      :u32 =    6;      // illegal opcode
pub const T_DEVICE     :u32 =    7;      // device not available
pub const T_DBLFLT     :u32 =    8;      // double fault
//pub const T_COPROC     :u32 =    9;      // reserved (not used since 486)
pub const T_TSS        :u32 =   10;      // invalid task switch segment
pub const T_SEGNP      :u32 =   11;      // segment not present
pub const T_STACK      :u32 =   12;      // stack exception
pub const T_GPFLT      :u32 =   13;      // general protection fault
pub const T_PGFLT      :u32 =   14;      // page fault
pub const T_RES        :u32 =   15;      // reserved
pub const T_FPERR      :u32 =   16;      // floating point error
pub const T_ALIGN      :u32 =   17;      // aligment check
pub const T_MCHK       :u32 =   18;      // machine check
pub const T_SIMDERR    :u32 =   19;      // SIMD floating point error

// These are arbitrarily chosen, but with care not to overlap
// processor defined exceptions or interrupt vectors.
pub const T_SYSCALL    :u32 =   64;      // system call
pub const T_DEFAULT    :u32 =  500;      // catchall

pub const T_IRQ0       :u32 =   32;      // IRQ 0 corresponds to int T_IRQ

pub const IRQ_TIMER    :u32 =    0;
pub const IRQ_KBD      :u32 =    1;
pub const IRQ_COM1     :u32 =    4;
pub const IRQ_IDE      :u32 =   14;
pub const IRQ_ERROR    :u32 =   19;
pub const IRQ_SPURIOUS :u32 =   31;

