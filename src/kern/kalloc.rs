use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{VirtAddr as VA};
use x86_64::structures::paging::Page;
use crate::*;

#[repr(transparent)]
struct Run {
    next: *mut Run,
}

#[repr(transparent)]
struct PhysPgAllocator {
    freelist: *mut Run,
}

// TODO: Change to Lock free structure
lazy_static! {
    static ref KMEM: Mutex<PhysPgAllocator> = Mutex::new(PhysPgAllocator{
        freelist: null_mut(),
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
        while p + PGSIZE < ed {
            self.kfree(p);
            p += PGSIZE;
        }
    }

    fn kfree(&mut self, v: VA) -> () {
        if !v.is_aligned(PGSIZE) || v.lt(&KERN_END) || v2p!(v.as_u64()) >= PHYSTOP {
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
        let r = self.freelist;
        unsafe {
            if !r.is_null() {
                self.freelist = (*r).next;
                VA::new(r as u64)
            } else{
                VA::new(0x0)
            }
        }
    }
}

unsafe impl Send for PhysPgAllocator {}

// Global allocator.
//unsafe impl GlobalAlloc for PhysPgAllocator {
//    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//        unimplemented!()
//    }
//
//    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//        unimplemented!()
//    }
//}

// Wrappers
pub fn kinit(st: VA, ed: VA) {
    KMEM.lock().kinit1(st, ed);
}

pub fn kfree(v: VA) -> () {
    KMEM.lock().kfree(v);
}

pub fn kalloc() -> Option<VA> {
    let v = KMEM.lock().kalloc();
    match v.as_u64() {
        0x0 => None,
        _ => Some(v),
    }
}

pub fn kalloc_pg() -> Option<Page> {
    let v = KMEM.lock().kalloc();
    match v.as_u64() {
        0x0 => None,
        _ => Some(Page::containing_address(v))
    }
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
    ($x:expr) => ($x - ($crate::DEVSPACE) + ($crate::DEVBASE))
}
