use crate::kern::kalloc::{kalloc};
use crate::*;
use crate::kern::proc::Proc;
use x86_64::ux::u9;
use x86_64::structures::paging::page_table::PageTable;
use x86_64::structures::paging::page_table::PageTableFlags as Flags;
use x86_64::structures::paging::page_table::PageTableEntry;
use core::ptr::Unique;


lazy_static! {
    static ref KPML4: Unique<PageTable> = {
        let pg = kalloc().expect("KMapper new: not enough mem");
        unsafe {
            memset(pg.as_mut_ptr() as *mut u8, 0, PGSIZE);

            let pml4 = pg.as_mut_ptr() as *mut PageTable;
            Unique::new_unchecked(pml4)
        }
    };
}

pub struct KMapper;
pub struct UMapper;

impl Mapper for KMapper {
    unsafe fn setup_vm(&self, p4: &mut PageTable) -> Result<(), &'static str> {
        // TODO: READ MAPPING FROM BOOTLOADER
        // Virtual address, physical start, physical end, flags
        // The KERN_BASE will be recognize as `struct KERN_BASE`, which
        // I do not understand. Maybe it has something to do with the lazy_static.
        let kd_u64 = KERN_DATA.align_up(PGSIZE).as_u64();
        let target_mapping: [(u64, u64, u64, Flags); 4] = [
            (KERN_BASE.as_u64(),          0,             EXTMEM,        Flags::WRITABLE),
            (KERN_BASE.as_u64() + EXTMEM, EXTMEM,        v2p!(kd_u64),  Flags::empty()),
            (kd_u64,                      v2p!(kd_u64),  PHYSTOP,       Flags::WRITABLE),
            (DEVBASE,                     DEVSPACE,      0x100000000,   Flags::WRITABLE)
        ];
        for k in target_mapping.iter() {
            let r = self.map(p4, VA::new(k.0), (k.2 - k.1) as usize, PA::new(k.1), k.3);
            if r.is_err() { return Err(r.unwrap_err()); }
        }

        Ok(())
    }

    fn switch_vm(&self) -> () {}
}

trait Mapper {
    unsafe fn setup_vm(&self, p4: &mut PageTable) -> Result<(), &'static str>;
    fn switch_vm(&self) -> ();

    // This function maps st
    unsafe fn map(&self, pg: &mut PageTable, st: VA, sz: usize, phys_addr: PA, flags: Flags)
        -> Result<(), &'static str> {
        let mut a = st.align_down(PGSIZE);
        let mut pa = phys_addr;
        let last = (a + (sz - 1)).align_down(PGSIZE);
        while a <= last {
            match self.walk(pg, a, 4, true) {
                Some(entry) => {
                    if entry.flags().contains(Flags::PRESENT) { panic!("remap"); }
                    entry.set_addr(pa, flags | Flags::PRESENT);
                }
                None => return Err("map failed")
            }
            a += PGSIZE;
            pa += PGSIZE;
        }

        Ok(())
    }

    unsafe fn walk<'a>(&self, pg: &'a mut PageTable, va: VA, lvl: u8, create: bool) -> Option<&'a mut PageTableEntry> {
        fn lvl_idx(pg: VA, lvl: u8) -> u9 {
            match lvl {
                1 => pg.p1_index(),
                2 => pg.p2_index(),
                3 => pg.p3_index(),
                4 => pg.p4_index(),
                _ => panic!("no such lvl")
            }
        }

        let entry = &mut pg[lvl_idx(va, lvl)];
        match entry.frame() {
            Ok(fr) => {
                if lvl > 1 {
                    let ptr_next_lvl = p2v!(fr.start_address().as_u64()) as *mut PageTable;
                    self.walk(&mut *ptr_next_lvl, va, lvl - 1, create)
                } else { Some(entry) }
            },
            Err(_) => {
                if create {
                    if lvl > 1 {
                        let new_page = kalloc().expect("walk: not enough mem");
                        let new_page_pa = PA::new(v2p!(new_page.as_u64()));
                        let ptr_next_lvl = new_page.as_mut_ptr() as *mut PageTable;
                        memset(new_page.as_mut_ptr() as *mut u8, 0, PGSIZE);
                        entry.set_addr(new_page_pa, Flags::PRESENT | Flags:: WRITABLE | Flags::USER_ACCESSIBLE);
                        self.walk(&mut *ptr_next_lvl, va, lvl - 1, create)
                    } else { Some(entry) }
                } else { None }
            },
        }
    }

    unsafe fn init_vm(&self, pml4: &mut PageTable, init: *const u64, sz: usize) -> () {
        if sz >= PGSIZE as usize { panic!("sz more than a page"); }
        let mem = kalloc().expect("Not enough mem!");
        let r = self.map(pml4, VA::zero(), sz, PA::new(v2p!(mem.as_u64())), Flags::WRITABLE | Flags::USER_ACCESSIBLE);
        if let Ok(_) = r {
            memmove(mem.as_mut_ptr() as *mut u8, init as *const u8, sz);
        } else { panic!("init_vm map failed!"); }
    }

    fn free_vm(&self) -> () {}
}

impl Mapper for UMapper {
    unsafe fn setup_vm(&self, _p4: &mut PageTable) -> Result<(), &'static str> { unimplemented!("setup_uvm not implemented") }
    fn switch_vm(&self) -> () {}
    fn free_vm(&self) -> () {}
}

pub unsafe fn kvm_alloc() -> () {
    let p4 = KPML4.as_ptr();
    match KMapper.setup_vm(&mut *p4) {
        Err(e) => panic!(e),
        _ => switch_kvm()
    }
}

pub unsafe fn switch_kvm() -> () {
    let p4 = KPML4.as_ptr();
    let value = v2p!(VA::from_ptr(p4).as_u64());
    asm!("mov $0, %cr3" :: "r" (value) : "memory");
}

pub unsafe fn switch_uvm(p: &Proc) -> () {
    let p4 = p.get_pml4();
    let value = VA::from_ptr(p4 as *const PageTable).as_u64();
    asm!("mov $0, %cr3" :: "r" (value) : "memory");
}

pub unsafe fn setup_kvm(pt: &mut PageTable) -> Result<(), &'static str> {
    KMapper.setup_vm(pt)
}

pub unsafe fn init_uvm(pml4: &mut PageTable, init: *const u64, sz: usize) {
    UMapper.init_vm(pml4, init, sz);
}