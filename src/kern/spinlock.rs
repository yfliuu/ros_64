use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;


pub struct SpinLock {
    lock: AtomicBool,
}


// Just a very simple spin lock.
impl SpinLock {
    pub const fn new() -> Self {
        SpinLock { lock: AtomicBool::new(false) }
    }

    pub fn acquire(&self) -> () {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) {
            unsafe { asm!("pause" : : : : "intel", "volatile"); }
        }
    }

    pub fn release(&self) -> () {
        self.lock.store(false, Ordering::Release);
    }
}
