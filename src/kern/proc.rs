use crate::*;
use x86_64::structures::idt::ExceptionStackFrame;
use kern::file::{File, INode};
use spin::Mutex;
use core::mem::uninitialized;

const NPROC: usize = 64;
const NO_FILE: usize = 16;

pub enum ProcState { UNUSED, EMBRYO, SLEEPING, RUNNABLE, RUNNING, ZOMBIE }

struct PTable {
    procs: &'static mut [Proc; NPROC],
}

lazy_static! {
    static ref PTABLE: Mutex<PTable> = Mutex::new(PTable::new());
}

#[allow(dead_code)]
struct Context {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    rbx: u64,
    rbp: u64,
    rip: u64,
}

#[allow(dead_code)]
struct Proc {
    sz: u64,                            // Size of process memory in bytes
    pml4: VA,                           // Page table
    kstack: VA,                         // Bottom of kernel stack for this process
    state: ProcState,                   // Process state
    pid: u64,                           // Process id
    parent: &'static Proc,              // Parent process
    ex_frame: ExceptionStackFrame,      // ESF for current syscall
    context: Context,                   // Context
    chan: VA,                           // If valid, sleeping on chan
    killed: bool,                       // If true, has been killed
    op_files: [File; NO_FILE],          // Opened files
    cwd: INode,                         // Current directory
    name: [char; 16],                   // Process name
}

impl PTable {
    fn new() -> Self {
        PTable { procs: unsafe { uninitialized() } }
    }
}

impl Proc {

}