#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points


use core::panic::PanicInfo;
use ros::{println, p2v, PHYSTOP, memmove};
use x86_64::VirtAddr as VA;
use ros::hlt_loop;


#[no_mangle] // don't mangle the name of this function
pub unsafe extern "C" fn kmain() -> ! {
    println!("Early init physical page allocator");
    ros::kern::kalloc::kinit(*ros::KERN_END, VA::new(p2v!(4*1024*1024 as u64)));

    println!("Initializing virtual memory");
    ros::kern::vm::kvm_alloc();

    println!("Initializing multi processor");
    ros::kern::mp::mp_init();

    println!("Initializing LAPIC");
    ros::kern::lapic::lapic_init();

    println!("Loading GDT & IDT");
    ros::kern::gdt64::gdt_init();
    ros::kern::idt::idt_init();

    println!("Initializing IOAPIC");
    ros::kern::ioapic::ioapic_init();

    println!("Initializing console");
    ros::kern::console::console_init();

    println!("Initializing UART");
    ros::kern::uart::uart_init();

    println!("Start other APs");
    // This does not work anymore by using #[thread_local]
    // ros::kern::mp::start_others();

    println!("Initializing memory");
    ros::kern::kalloc::kinit(VA::new(p2v!(4*1024*1024 as u64)), VA::new(p2v!(PHYSTOP)));

    println!("User space initialization");
    ros::kern::proc::user_init();

    println!("Ready to run scheduler");
    mp_main();

    println!("FATAL ERROR: DROP TO MAIN HLT LOOP");
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop()
}

#[no_mangle]
unsafe fn mp_enter() {
    ros::kern::vm::switch_kvm();
    ros::kern::gdt64::gdt_init();
    ros::kern::lapic::lapic_init();
    mp_main();
}

unsafe fn mp_main() -> ! {
    ros::kern::proc::scheduler();
}
