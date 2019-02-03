#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(asm)]

use core::panic::PanicInfo;
use ros::println;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn kmain() -> ! {
    println!("HELLO W");
    println!("HELLO WORLD2");
    println!("HELLO WORLD3");
    println!("HELLO WORLD4");
    forever();
}

#[no_mangle]
fn forever() -> ! {
    loop{}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
