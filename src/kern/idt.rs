// The x86-interrupt calling convention leads to the following LLVM error
// when compiled for a Windows target: "offset is not a multiple of 16". This
// happens for example when running `cargo test` on Windows. To avoid this
// problem we skip compilation of this module on Windows.
#![cfg(not(windows))]

use crate::*;
use x86_64::structures::idt::{InterruptStackFrame, InterruptDescriptorTable, PageFaultErrorCode};
use x86_64::PrivilegeLevel;
use crate::kern::lapic::lapic_eoi;
use crate::kern::mp::my_cpu;
use crate::kern::spinlock::SpinLock;
use crate::kern::proc::wakeup;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(0);
        }
        idt.divide_by_zero.set_handler_fn(divide_by_zero);
        idt.debug.set_handler_fn(debug_trap);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt);
        idt.breakpoint.set_handler_fn(break_point);
        idt.overflow.set_handler_fn(overflow);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded);
        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.device_not_available.set_handler_fn(device_not_available);
        idt.invalid_tss.set_handler_fn(invalid_tss);
        idt.segment_not_present.set_handler_fn(segment_not_present);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault);
        idt.general_protection_fault.set_handler_fn(general_protection_fault);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.alignment_check.set_handler_fn(alignment_check);
        idt[(T_IRQ0 + IRQ_TIMER) as usize].set_handler_fn(timer_interrupt_handler);
        idt[(T_IRQ0 + IRQ_KBD)   as usize].set_handler_fn(keyboard_interrupt_handler);

        idt[T_SYSCALL as usize].set_handler_fn(syscall).set_privilege_level(PrivilegeLevel::Ring3);
        idt
    };
}

static TICKSLOCK: SpinLock = SpinLock::new();
static mut ticks: u64 = 0;

pub fn idt_init() {
    IDT.load();
}

extern "x86-interrupt" fn divide_by_zero(_stack_frame: &mut InterruptStackFrame) {
    println!("DIVIDE BY ZERO FAULT\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn debug_trap(_stack_frame: &mut InterruptStackFrame) {
    println!("DEBUG TRAP\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn non_maskable_interrupt(_stack_frame: &mut InterruptStackFrame) {
    println!("NON MASKABLE INTERRUPT\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn break_point(_stack_frame: &mut InterruptStackFrame) {
    println!("BREAK POINT\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn overflow(_stack_frame: &mut InterruptStackFrame) {
    println!("OVERFLOW\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn bound_range_exceeded(_stack_frame: &mut InterruptStackFrame) {
    println!("BOUND RANGE EXCEEDED\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_opcode(_stack_frame: &mut InterruptStackFrame) {
    println!("INVALID OPCODE\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn device_not_available(_stack_frame: &mut InterruptStackFrame) {
    println!("DEVICE NOT AVAILABLE\n{:#?}", _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_tss(_stack_frame: &mut InterruptStackFrame, _error_code: u64) {
    println!("INVALID TSS, error_code: {}\n{:#?}", _error_code, _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn segment_not_present(_stack_frame: &mut InterruptStackFrame, _error_code: u64) {
    println!("SEGMENT NOT PRESENT, error_code: {}\n{:#?}", _error_code, _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn stack_segment_fault(_stack_frame: &mut InterruptStackFrame, _error_code: u64) {
    println!("STACK SEGMENT FAULT, error_code: {}\n{:#?}", _error_code, _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT: err_code: {:x}\n{:#?}", _error_code, _stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn alignment_check(_stack_frame: &mut InterruptStackFrame, _error_code: u64) {
    println!("ALIGNMENT CHECK: err_code: {}\n{:#?}", _error_code, _stack_frame);
    hlt_loop();
}


extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
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

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    unsafe {
        if my_cpu().id == 0 {
            TICKSLOCK.acquire();
            ticks += 1;
            let ticks_addr = &ticks as *const u64;
            wakeup(VA::from_ptr(ticks_addr));
            TICKSLOCK.release();
        }
    }
    lapic_eoi();
}

extern "x86-interrupt" fn syscall(_stack_frame: &mut InterruptStackFrame) {
    
}