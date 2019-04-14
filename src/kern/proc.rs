use crate::*;
use x86_64::structures::idt::{InterruptStackFrame};
use kern::file::{File, INode};
use kern::lapic::sti;
use crate::kern::spinlock::SpinLock;
use crate::kern::kalloc::kalloc;
use crate::kern::mp::my_cpu;
use crate::kern::vm::*;
use core::ptr::Unique;
use core::borrow::{BorrowMut};
use x86_64::structures::paging::page_table::PageTable;
use array_init::array_init;
use core::mem::{MaybeUninit, uninitialized};
use core::default::Default;

const NPROC: usize = 32;
const NO_FILE: usize = 16;
const KSTACKSIZE: u64 = 4096;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ProcState { UNUSED, EMBRYO, SLEEPING, RUNNABLE, RUNNING, ZOMBIE }

lazy_static! {
    static ref PTABLE: Unique<PTable> = unsafe {
        Unique::new_unchecked(PTable::new_ptr())
    };
}

// We cannot wrap the entire PTable with mutex, we need some fine grained sync.
static PTLOCK: SpinLock = SpinLock::new();
static mut NEXT_PID: usize = 1;

// Process table.
// All modification of process should be routed through PTable struct.
// Processes are owned by PTable.
struct PTable {
    procs: [Proc<'static>; NPROC],
}

impl PTable {
    unsafe fn new_ptr() -> *mut PTable {
        // We cannot create PTable on the stack.
        // because the kernel stack will blow (it took me a whole day to figure out).
        // We have only 8kb of kernel stack.
        // We need to create the PTable on the heap.
        let num_pages = size_of::<PTable>() / PGSIZE as usize + 1;
        let mut addr = VA::zero();
        for _ in 0..num_pages {
            addr = kalloc().expect("NOT ENOUGH MEM FOR PTABLE!");
        }
        let ptr = addr.as_mut_ptr::<PTable>();
        let pt = &mut *ptr;
        for (i, item) in pt.procs.iter_mut().enumerate() {
            item.init(i);
        }
        ptr
    }

    pub unsafe fn alloc_proc(&mut self) -> Option<&'static mut Proc> {
        PTLOCK.acquire();

        match self.procs.iter_mut().find(|x|{(**x).is_unused()}) {
            Some(p) => {
                p.state = ProcState::EMBRYO;
                p.pid = NEXT_PID; NEXT_PID += 1;

                // We change the state to EMBRYO and it's safe to release the lock
                PTLOCK.release();
                match kalloc() {
                    Some(v) => p.kstack = v,
                    None => { p.state = ProcState::UNUSED; return None;}
                }

                let mut sp = p.kstack + KSTACKSIZE;
                sp -= size_of::<InterruptStackFrame>();
                p.tf = Some(&mut *(sp.as_mut_ptr::<InterruptStackFrame>()));
                sp -= 4 as usize;

                // TODO: trap return and fork return
                *sp.as_mut_ptr() = trap_ret;
                sp -= size_of::<Context>();
                let ctx = &mut *(sp.as_mut_ptr::<Context>());
                ctx.clear();
                ctx.set_rip(VA::from_ptr(fork_ret as *const ()).as_u64() as usize);
                p.context = Some(ctx);
                Some(p)
            }
            None => {
                PTLOCK.release();
                None
            }
        }
    }


}

pub struct Proc<'a> {
    sz: u64,                                    // Size of process memory in bytes
    pml4: Option<&'a mut PageTable>,            // Page table
    kstack: VA,                                 // Bottom of kernel stack for this process
    state: ProcState,                           // Process state
    pid: usize,                                 // Process id
    parent: Option<&'a Proc<'a>>,               // Parent process
    tf: Option<&'a mut InterruptStackFrame>,    // ESF for current syscall
    context: Option<&'a mut Context>,           // Context
    chan: VA,                                   // If valid, sleeping on chan
    killed: bool,                               // If true, has been killed
    op_files: [Option<File>; NO_FILE],          // Opened files
    cwd: Option<INode>,                         // Current directory
    name: &'static str,                         // Process name
}

impl<'a> Proc<'a> {
    fn new(pid: usize) -> Self { Proc {
            sz: 0,
            pml4: None,
            kstack: VA::zero(),
            state: ProcState::UNUSED,
            pid: pid,
            parent: None,
            tf: None,
            context: None,
            chan: VA::zero(),
            killed: false,
            op_files: [None; NO_FILE],
            cwd: None,
            name: "",
    } }

    fn init(&mut self, pid: usize) -> () {
        self.sz = 0;
        self.pml4 = None;
        self.state = ProcState::UNUSED;
        self.pid = pid;
        self.kstack = VA::zero();
        self.parent = None;
        self.tf = None;
        self.context = None;
        self.chan = VA::zero();
        self.killed = false;
        self.op_files = [None; NO_FILE];
        self.cwd = None;
        self.name = "";
    }

    pub fn is_unused(&self) -> bool { self.state == ProcState::UNUSED }

    pub fn get_pml4(&self) -> &PageTable {
        if let Some(ref x) = self.pml4 {
            &x
        } else { panic!("PML4 empty!") }
    }

    pub fn get_mut_pml4(&mut self) -> &mut PageTable {
        match self.pml4 {
            Some(ref mut x) => x,
            None => panic!("PML4 empty!")
        }
    }

    pub fn get_mut_ctx(&mut self) -> &mut Context {
        if let Some(ref mut ctx) = self.context {
            *ctx
        } else { panic!("Context empty!"); }
    }

    pub fn get_ctx(&self) -> &Context {
        if let Some(ref ctx) = self.context {
            ctx
        } else { panic!("Context empty"); }
    }

    pub fn set_state(&mut self, state: ProcState) -> () {
        self.state = state;
    }

    pub fn get_parent(&self) -> &Proc {
        if let Some (ref x) = self.parent {
            *x
        } else { panic!("Parent empty!"); }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Context {
    r11: usize,
    r12: usize,
    r13: usize,
    r14: usize,
    r15: usize,
    rbx: usize,
    rbp: usize,
    rip: usize
}

impl Context {
    pub const fn new() -> Context { Context {
        r11: 0,
        r12: 0,
        r13: 0,
        r14: 0,
        r15: 0,
        rbx: 0,
        rbp: 0,
        rip: 0
    } }

    pub fn set_rip(&mut self, addr: usize) -> () {
        self.rip = addr;
    }

    pub fn clear(&mut self) -> () {
        self.rbx = 0;
        self.r11 = 0;
        self.r12 = 0;
        self.r13 = 0;
        self.r14 = 0;
        self.r15 = 0;
        self.rbp = 0;
        self.rbx = 0;
        self.rip = 0;
    }
}

pub unsafe fn user_init() -> () {
    let ptr = PTABLE.as_ptr();
    let mut p: &'static mut Proc = (*ptr).alloc_proc().expect("alloc_proc failed");

    // TODO: _bin_init_code and sz

    let ptr = kalloc().expect("Allocate failed!").as_mut_ptr::<PageTable>();
    memset(ptr, 0, 4096);
    p.pml4 = Some(&mut *ptr);
    let pml4 = p.get_mut_pml4();

    setup_kvm(pml4).ok().expect("kvm setup failed");
    init_uvm(pml4,
             (*BINARY_INITCODE_START).as_ptr(),
             (*BINARY_INITCODE_SIZE).as_u64() as usize);

    p.sz = PGSIZE;
    p.state = ProcState::RUNNABLE;

    if let Some(ref mut non_mut_tf) = p.tf {
        let tf = (*non_mut_tf).as_mut();
        tf.instruction_pointer = VA::zero();
        tf.stack_pointer = VA::new(PGSIZE);
        tf.cpu_flags = 0x200;    // Interrupt enabled
    }
}

pub unsafe fn scheduler() -> ! {
    let cpu = my_cpu();

    loop {
        // Enabling interrupts on this cpu
        sti();
        PTLOCK.acquire();
        for p in (*PTABLE.as_ptr()).procs.iter_mut() {
            if p.state != ProcState::RUNNABLE { continue }
            cpu.set_proc(p);
            switch_uvm(cpu.get_proc());

            cpu.set_proc_state(ProcState::RUNNING);

            switch(&&cpu.scheduler, &cpu.get_proc().get_ctx());
            switch_kvm();

            cpu.clear_proc();
        }
        PTLOCK.release();
    }
}

#[naked]
pub unsafe extern "C" fn trap_ret() {
    asm!("pop %rax");
    asm!("pop %rbx");
    asm!("pop %rcx");
    asm!("pop %rdx");
    asm!("pop %rbp");
    asm!("pop %rsi");
    asm!("pop %rdi");
    asm!("pop %r8");
    asm!("pop %r9");
    asm!("pop %r10");
    asm!("pop %r11");
    asm!("pop %r12");
    asm!("pop %r13");
    asm!("pop %r14");
    asm!("pop %r15");
    asm!("add $$16, %rsp");
    asm!("iret");
}

pub fn fork_ret() -> () {
    // We still holds the PTLOCK.
    PTLOCK.release();

    // we could do some init stuff here
}

#[naked]
pub unsafe extern "C" fn switch(_old: &&Context, _new: &&Context) {
    // Save old callee-save registers
    asm!("push %rbp");
    asm!("push %rbx");
    asm!("push %r11");
    asm!("push %r12");
    asm!("push %r13");
    asm!("push %r14");
    asm!("push %r15");
    
    // Switch stacks
    asm!("mov %rsp, (%rdi)");
    asm!("mov %rsi, %rsp");
    
    // Load new callee-save registers
    asm!("pop %r15");
    asm!("pop %r14");
    asm!("pop %r13");
    asm!("pop %r12");
    asm!("pop %r11");
    asm!("pop %rbx");
    asm!("pop %rbp");
    
    asm!("ret");
}
