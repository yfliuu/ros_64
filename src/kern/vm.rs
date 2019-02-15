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
        *pml4.offset(511) = v2p!(ptr2u64(pdpt)) | PTE_P | PTE_W;

        // 2GB (1GB per entry) directory pointer table entry
        *pdpt.offset(511) = v2p!(ptr2u64(pgdir1)) | PTE_P | PTE_W;
        *pdpt.offset(510) = v2p!(ptr2u64(pgdir0)) | PTE_P | PTE_W;

        // IO pgdir
        *pdpt.offset(509) = v2p!(ptr2u64(iopgdir)) | PTE_P | PTE_W;

        // map to 0x0
        for n in 0..ENTRY_COUNT {
            *pgdir0.offset(n as isize) = (n << 22) as u64 | PTE_P | PTE_W | PTE_PS;
            *pgdir1.offset(n as isize) = ((n + 512) << 22) as u64 | PTE_P | PTE_W | PTE_PS;
        }

        // map device mem to physical address 0xfe000000
        for n in 0..16 {
            *iopgdir.offset(n) = (DEVSPACE + ((n as u64) << 22)) | PTE_PS | PTE_P | PTE_W | PTE_PWT | PTE_PCD;
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
