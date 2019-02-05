#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(asm)]

use core::panic::PanicInfo;
use ros::{println, UART};

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn kmain() -> ! {

    println!("Initializing UART");
    UART.init();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
