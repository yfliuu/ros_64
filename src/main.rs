#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

// static HELLO: &[u8] = b"Hello World!";

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn kmain() -> ! {
//    let vga_buffer = phy32_to_64va(0xb8000) as *mut u16;
//    unsafe {
//        *vga_buffer.offset(640 as isize) = 0x769;
//         for (i, &byte) in HELLO.iter().enumerate() {
//             {
//                 *vga_buffer.offset(i as isize * 2) = byte;
//                 *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
//             }
//         }
//    }


    forever();
}

fn phy32_to_64va(pha: u32) -> u64 {
    const PHY32_OFFSET: u64 = 0xFFFFFEFF00000000;
    pha as u64 + PHY32_OFFSET
}

#[no_mangle]
fn forever() -> ! {
    loop{}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
