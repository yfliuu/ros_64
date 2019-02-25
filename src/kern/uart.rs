use x86_64::instructions::port::Port;
use crate::kern::ioapic::ioapic_enable;
use crate::*;

// UART. The SerialPort::new is a const fn.
const COM1: u16 = 0x3f8;
pub const UART: uart_16550::SerialPort = uart_16550::SerialPort::new(COM1);

pub unsafe fn uart_init() -> () {
    UART.init();
    let _: u8 = Port::new(COM1 + 2u16).read();
    let _: u8 = Port::new(COM1 + 0u16).read();
    ioapic_enable(IRQ_COM1, 0);
}
