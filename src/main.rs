#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(asm)]

use core::panic::PanicInfo;
use ros::{println, UART, p2v};
use ros::kern::kalloc::kinit1;
use ros::kern::vm::kvmalloc;
use ros::kern::mp::mpinit;
use x86_64::VirtAddr as VA;


#[no_mangle] // don't mangle the name of this function
pub extern "C" fn kmain() -> ! {

    println!("Initializing UART");
    UART.init();

    println!("Initializing physical page allocator");
    kinit1(*ros::KERN_END, VA::new(p2v!(4*1024*1024 as u64)));

    println!("Initializing virtual memory");
    kvmalloc();

    println!("Initializing multi processor");
    mpinit();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
