use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{VirtAddr as VA};
use crate::{PGSIZE, memset};

#[repr(transparent)]
struct Run {
    next: *mut Run,
}

#[repr(transparent)]
struct PhysPgAllocator {
    freelist: *mut Run,
}

lazy_static! {
    static ref KMEM: Mutex<PhysPgAllocator> = Mutex::new(PhysPgAllocator{
        freelist: 0x0 as *mut Run,
    });
}

#[allow(dead_code)]
impl PhysPgAllocator {
    // Initialization, phase 1
    fn kinit1(&mut self, st: VA, ed: VA) -> () {
        self.free_range(st, ed);
    }

    fn free_range(&mut self, st: VA, ed: VA) -> () {
        let  mut p = st.align_up(PGSIZE);
        while p + PGSIZE <= ed {
            self.kfree(p);
            p += PGSIZE;
        }
    }

    fn kfree(&mut self, v: VA) -> () {
        // TODO: Here we should check that v is in the safe range
        if !v.is_aligned(PGSIZE) {
            panic!("kfree")
        }
        let r = v.as_mut_ptr::<Run>();
        unsafe {
            memset(r, 1, PGSIZE);
            (*r).next = self.freelist;
        }
        self.freelist = r;
    }

    fn kalloc(&mut self) -> VA {
        let r = self.freelist as *mut Run;
        unsafe {
            if r.ne(&(0x0 as *mut Run)) {
                self.freelist = (*r).next;
            }
        }
        VA::new(r as u64)
    }
}

unsafe impl Send for PhysPgAllocator {}

// Wrappers
pub fn kinit1(st: VA, ed: VA) {
    KMEM.lock().kinit1(st, ed);
}

pub fn kfree(v: VA) -> () {
    KMEM.lock().kfree(v);
}

pub fn kalloc() -> VA {
    KMEM.lock().kalloc()
}

#[macro_export]
macro_rules! p2v {
    ($x:expr) => ($x + (*$crate::KERN_BASE).as_u64());
}

#[macro_export]
macro_rules! v2p {
    ($x:expr) => ($x - (*$crate::KERN_BASE).as_u64());
}

#[macro_export]
macro_rules! io2v {
    ($x:expr) => ($x + ($crate::DEVBASE) - ($crate::DEVSPACE))
}
