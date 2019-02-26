#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points


use core::panic::PanicInfo;
use ros::{println, p2v};
use x86_64::VirtAddr as VA;
use ros::hlt_loop;


#[no_mangle] // don't mangle the name of this function
pub unsafe extern "C" fn kmain() -> ! {
    println!("Early init physical page allocator");
    ros::kern::kalloc::kinit1(*ros::KERN_END, VA::new(p2v!(4*1024*1024 as u64)));

    println!("Initializing virtual memory");
    ros::kern::vm::kvm_alloc();

    println!("Initializing multi processor");
    ros::kern::mp::mp_init();

    println!("Initializing LAPIC");
    ros::kern::lapic::lapic_init();

    println!("Loading GDT & IDT");
    ros::kern::gdt::gdt_init();
    ros::kern::idt::idt_init();

    println!("Initializing IOAPIC");
    ros::kern::ioapic::ioapic_init();

    println!("Initializing console");
    ros::kern::console::console_init();

    println!("Initializing UART");
    ros::kern::uart::uart_init();

    ros::kern::lapic::sti();
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop()
}
