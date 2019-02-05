#![no_std]

pub mod kern;

// UART. The SerialPort::new is a const fn.
const COM1: u16 = 0x3f8;
pub const UART: uart_16550::SerialPort = uart_16550::SerialPort::new(COM1);