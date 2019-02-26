// The x86-interrupt calling convention leads to the following LLVM error
// when compiled for a Windows target: "offset is not a multiple of 16". This
// happens for example when running `cargo test` on Windows. To avoid this
// problem we skip compilation of this module on Windows.
#![cfg(not(windows))]

use crate::*;
use x86_64::structures::idt::{ExceptionStackFrame, InterruptDescriptorTable, PageFaultErrorCode};
use x86_64::PrivilegeLevel;
use crate::kern::lapic::lapic_eoi;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(0);
        }
        idt.general_protection_fault.set_handler_fn(general_protection_fault);
        idt[(T_IRQ0 + IRQ_TIMER) as usize].set_handler_fn(timer_interrupt_handler);
        idt[(T_IRQ0 + IRQ_KBD)   as usize].set_handler_fn(keyboard_interrupt_handler);

        idt[T_SYSCALL as usize].set_handler_fn(syscall).set_privilege_level(PrivilegeLevel::Ring3);
        idt
    };
}

pub fn idt_init() {
    IDT.load();
}

extern "x86-interrupt" fn general_protection_fault(
    _stack_frame: &mut ExceptionStackFrame,
    _error_code: u64) {
    println!("GPF\n    ip: {:x}\n    cs: {:x}\n    flags: {:x}\n    ss: {:x}\n    sp: {:x}\n    err_code: 0x{:x}",
             _stack_frame.instruction_pointer.as_u64(),
             _stack_frame.code_segment,
             _stack_frame.cpu_flags,
             _stack_frame.stack_segment,
             _stack_frame.stack_pointer.as_u64(),
             _error_code);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    _error_code: u64,
) {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    _error_code: PageFaultErrorCode,
) {
    use crate::hlt_loop;
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut ExceptionStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }
    let mut keyboard = KEYBOARD.lock();
    let port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }
    lapic_eoi();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut ExceptionStackFrame) {
    lapic_eoi();
}

extern "x86-interrupt" fn syscall(_stack_frame: &mut ExceptionStackFrame) {
    
}