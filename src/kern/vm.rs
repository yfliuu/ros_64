use crate::kern::kalloc::kalloc;
use crate::*;

#[repr(C)]
struct KernVM {
    kpml4: VA,
    kpdpt: VA,
    kpgdir0: VA,
    kpgdir1: VA,
    iopgdir: VA,
}

lazy_static! {
    static ref KVM: KernVM = init_vm();
}

fn init_vm() -> KernVM {
    // These memory allocations should not fail. Panic on unwrap if failed.
    let pml4 = kalloc().unwrap().as_mut_ptr::<u64>();
    let pdpt = kalloc().unwrap().as_mut_ptr::<u64>();
    let pgdir0 = kalloc().unwrap().as_mut_ptr::<u64>();
    let pgdir1 = kalloc().unwrap().as_mut_ptr::<u64>();
    let iopgdir = kalloc().unwrap().as_mut_ptr::<u64>();

    memset(pml4, 0, PGSIZE);
    memset(pdpt, 0, PGSIZE);
    memset(iopgdir, 0, PGSIZE);

    unsafe {
        // Linear map the first 2GB of physical memory starting at
        // 0xffffffff80000000 to 0x0

        // 512GB per entry, 0xffffffff80000000 is at the start of the mapping of last entry
        *pml4.offset(511) = v2p!(VA::from_ptr(pdpt).as_u64()) | PTE_P | PTE_W;

        // 2GB (1GB per entry) directory pointer table entry
        *pdpt.offset(511) = v2p!(VA::from_ptr(pgdir1).as_u64()) | PTE_P | PTE_W;
        *pdpt.offset(510) = v2p!(VA::from_ptr(pgdir0).as_u64()) | PTE_P | PTE_W;

        // IO pgdir
        *pdpt.offset(509) = v2p!(VA::from_ptr(iopgdir).as_u64()) | PTE_P | PTE_W;

        // map to 0x0. Marking PTE_PS to turn it into huge page (2MB) so
        // we don't have to go a level deeper to map a bunch of page table entries.
        // 4k seems too small for 64bit arch. But anyway we allocate 4k "page"
        // in our physical allocator.
        //
        // We left shift 21 bits (instead of 22 bits on x86) here
        // because on 64bit arch it's 9+9+9+9+12.
        // If you're porting some code from 32bit then be careful.
        // This bug won't cause you any trouble until you programming lapic and ioapic.
        // The println is normal and everything seems normal.
        // It took me quite a long time to figure out.
        for n in 0..ENTRY_COUNT {
            *pgdir0.offset(n as isize) = (n << 21) as u64 | PTE_P | PTE_W | PTE_PS;
            *pgdir1.offset(n as isize) = ((n + 512) << 21) as u64 | PTE_P | PTE_W | PTE_PS;
        }

        // map device mem to physical address 0xfe000000
        for n in 0..16 {
            *iopgdir.offset(n) = (DEVSPACE + ((n as u64) << 21)) | PTE_PS | PTE_P | PTE_W | PTE_PWT | PTE_PCD;
        }
    }

    KernVM {
        kpml4: VA::from_ptr(pml4),
        kpdpt: VA::from_ptr(pdpt),
        kpgdir1: VA::from_ptr(pgdir1),
        kpgdir0: VA::from_ptr(pgdir0),
        iopgdir: VA::from_ptr(iopgdir),
    }
}

pub fn kvm_alloc() -> () {
    switch_kvm();
}

fn switch_kvm() -> () {
    let value = v2p!(KVM.kpml4.as_u64());
    unsafe {
        asm!("mov $0, %cr3" :: "r" (value) : "memory");
    }
}
