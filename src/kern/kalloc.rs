use spin::Mutex;
use x86_64::{PhysAddr as PA, VirtAddr as VA};


extern "C" {
    static KERNEL_END: u64;
}

struct Run<'a> {
    next: &'a Run<'a>,
}

struct PhysPgAllocator<'a> {
    freelist: &'a Run<'a>,
}

static mut KMem: Mutex<PhysPgAllocator> = Mutex::new(KMem{
    freelist: Nil,
});

impl PhysPgAllocator {
    const PGSIZE: u64 = 4096;

    // Initialization, phase 1
    pub fn kinit1(&mut self, st: VA, ed: VA) -> () {
        self.free_range(st, ed);
    }

    pub fn free_range(&mut self, st: VA, ed: VA) -> () {
        let p = st.align_up(PGSIZE);
        while p + PGSIZE <= ed {
            self.kfree(p);
            p += PGSIZE;
        }
    }

    pub fn kfree(&mut self, v: VA) -> () {

        // TODO: Here we should check that v is in the safe range
        if !v.is_aligned(PGSIZE) {
            panic("kfree")
        }

        let r = v.as_mut_ptr::<Run>();
    }
}