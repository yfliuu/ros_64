use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::port::Port;
use crate::memmove;
use core::mem::size_of;
use crate::memset;

const VGA_BUFFER: u64 = 0xffffffff800b8000;
const CRT_PORT: u16 = 0x3d4;
const BACKSPACE: u8 = 0x08;

lazy_static! {
    /// A global `Writer` instance that can be used for printing to the VGA text buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(VGA_BUFFER as *mut Buffer) },
        crt_p1: Port::new(CRT_PORT),
        crt_p2: Port::new(CRT_PORT + 1),
    });
}

/// The standard color palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A screen character in the VGA text buffer, consisting of an ASCII character and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// The height of the text buffer (normally 25 lines).
const BUFFER_HEIGHT: usize = 25;
/// The width of the text buffer (normally 80 columns).
const BUFFER_WIDTH: usize = 80;

/// A structure representing the VGA text buffer.
struct Buffer {
    chars: [Volatile<ScreenChar>; BUFFER_WIDTH * BUFFER_HEIGHT],
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the
/// `core::fmt::Write` trait.
pub struct Writer {
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    crt_p1: Port<u8>,
    crt_p2: Port<u8>,
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) { unsafe {
        self.crt_p1.write(14);
        let mut pos: u32 = (self.crt_p2.read() as u32) << 8;
        self.crt_p1.write(15);
        pos |= self.crt_p2.read() as u32;

        match byte {
            b'\n' => pos += 80 - pos % 80,
            BACKSPACE => if pos > 0 { pos -= 1; }
            c => {
                self.buffer.chars[pos as usize].write(ScreenChar {
                    ascii_character: c as u8,
                    color_code: self.color_code,
                });
                pos += 1;
            }
        }

        let crt = self.buffer.chars.as_mut_ptr() as *mut u16;

        // Scroll up.
        if (pos / 80) >= 24 {
            memmove::<u16>(crt,
                           crt.offset(80),
                           (size_of::<ScreenChar>() * 23 * 80) as usize);
            pos -= 80;
            memset(crt.offset(pos as isize),
                   0,
                   (size_of::<ScreenChar>() * (24 * 80 - pos) as usize) as u64);
        }

        self.crt_p1.write(14);
        self.crt_p2.write((pos >> 8) as u8);
        self.crt_p1.write(15);
        self.crt_p2.write(pos as u8);

        self.buffer.chars[pos as usize].write(ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        });
    } }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character. Does **not**
    /// support strings with non-ASCII characters, since they can't be printed in the VGA text
    /// mode.
    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20...0x7e | b'\n' | BACKSPACE => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::kern::console::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

pub fn console_init() -> () {
    use crate::kern::ioapic::ioapic_enable;
    // TODO: Link console read/write to stdin/out
    ioapic_enable(crate::IRQ_KBD, 0);
}