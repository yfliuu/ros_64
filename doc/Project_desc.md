# CSCE-613 Project documentation

## Introduction
This is the design document for CSCE-613 Operating system. The OS is named ros
(Rust OS). The following sections will briefly describe the design and implementation
of ros.

## Bootloader
Ros does not use the bootloader provided by blog_os by default. Instead it uses the
modified bootloader of [xv6](https://github.com/mit-pdos/xv6-public) kernel. It 
is multiboot compatible, which means that it can also be booted using GRUB. Various
booting command is described in project README.md file.

The 64bit boot process is described as follows:
* Power on
* Machine starts in 16bit real mode
* The 16bit boot code does the following before jumping to 32bit protected mode
    * Enable A20 line
    * Load GDT
    * Setup segment registers
    * Setup stack and call to C
* Bootloader main code (C) loads the kernel into memory
* Call the kernel entry code (32bit)

Although the kernel is compiled in 64bit mode, it has to have a 32bit bootstrap
code that bootloader can call to. This 32bit bootstrap code is also the starting point
if using a multiboot-compatible bootloader.

After bootloader handles control to the 32bit bootstrap code of kernel,
the bootstrap code does the following:
* Multiboot compatible bootloader will pass some useful information about machine,
save it first.
* Check that CPU supports long mode (via `cpuid` instruction)
* Test the IA-32e bit
* Setup state for long mode. This includes:
    * Enable PGE (Page Global Enable)
    * Enable PAE (Physical Address Extension)
    * Enable PSE (Page Size Extensions)
* Load PML4 (Page mapping level 4, analogous to the Page Directory in x86)
* Enable IA-32e mode
* Enable Paging and enters long mode

At this point the code cannot jump directly to a 64bit high address because the `jmp`
instruction is still in 32bit code block. We can only jump to a 64bit low address,
and jump again from there to 64bit high address.

The code at 64bit low address will do the following:
* Load GDT
* Jump to 64bit high address

After we reached the 64bit high address we can do similar things as x86:
* Clear low memory mapping
* Setup segment registers
* Setup stack pointer
* Call the Rust code

After this the control is transferred to Rust code.

Recall that the mapping is mandatory for entering long mode. The bootloader used in
blog_os uses an identity mapping where the bootloader here linearly maps 
two 2MB huge pages starting from ```0xffffffff80000000.```

## Physical Page Allocator
The PPA (Physical Page Allocator) used in `ros` is a simple linked list allocator.
This means that all free pages are linked to one linked list. It takes constant time
to allocate or free a page, and it did not require additional storage because link
node is stored inside the free pages. The code of PPA is in `kalloc.rs` in `src/kern`.

Using linked list allocator is simple but it requires that
all pages be mapped before it can allocate pages. So a problem arises: In order to
map pages, we need free pages to store page tables. But in order to allocate free
pages, we need to map all pages.

The solution to this problem is to do mapping in two phase. First map enough (4MB) 
pages in bootstrap code so that we can setup our PPA on top of these pages.
After that, map all physical pages and extend our PPA to the entire physical address
space.

The linked list allocator approach has one significant drawback: setting up the PPA
requires that all physical pages be mapped. This means that if we don't use the huge
pages, we'll have to setup over one million pointers at startup if we have 4GB of 
memory. A possible solution is to use some additional data structures to manage free
pages. This is also a future work for `ros`.

## Virtual Memory
Ros did not use the virtual memory module provided by blog_os. Instead it 
uses a similar approach (modified to 64bit) as of xv6 kernel (`vm.rs`). After setting up the PPA, 
the kernel recursively maps all the kernel memory. 

Recursive page tables is not used
here because ros uses a linear mapping: given a physical address, add `KERNEL_BASE` to
it to translate it to the corresponding virtual address. Similar rules applies to
virtual to physical translation (minus `KERNEL_BASE`). The advantage of this approach
is that the addresses are predictable, which means that it is easy to reason and program
in this form of address translation.

One of the drawback of this recursive approach is that initial mapping is much slower than
x86 because recursive can be deep on x86_64 (four levels of paging structure vs.
two in x86). Currently I do not have a good solution to this problem other than using
huge pages. And unfortunately, the `x86_64` crate (written by Phil) explicitly forbids
the use of huge pages and it contains convenient structures for manipulating paging
structures.

## Multicore (W.I.P)
Full multicore support (for scheduling) is still work in progress. 

The multicore boot process is described as follows:
* Bootstrap core (BSP, chosen by BIOS) start
* Perform necessary global initialization (PPA, VM, etc.)
* Perform Universal startup algorithm, which includes
    * Send INIT (level-triggered) interrupt to reset other CPU
    * Send startup IPI twice to enter code
* Wait for APs (Application core) to start
* Bootstrap finished

In order to detect the number of cores on the machine, the BSP does the following after startup:
* Search for MP floating pointer structure, which, according to the manual, 
is located in one of the three places:
    * in the first KB of the EBDA
    * in the last KB of system base memory
    * in the BIOS ROM between 0xE0000 and 0xFFFFF (In QEMU it will typically be found here)
* Calculating checksum
* Read the MP Floating Pointer structure. More precisely, read:
    * CPUs
    * APIC IDs
    * IOAPIC MMIO address
    
After a successful core detection, the BSP is ready to bootstrap all other APs. It will do the following:
* Load the AP boot code into memory (same address as BSP)
* Pass the following arguments to AP, by copying the address of arguments to a reachable low address
for AP (for example, from starting address 0x7000 backwards). AP starts in 16bit real mode.
    * Kernel stack
    * Jump address (the Rust code to call into when successfully booted)
    * Page Mapping Level 4
* Perform startup algorithm described above
* Repeat for every AP

AP will perform its necessary per-core intialization (IDT, GDT, LAPIC, etc.) after startup.

**The part that was W.I.P**

Multicore support for threading is still WIP. Some of the difficulties are:

Rust allows the use of `#[thread_local]` derivitive for per-core storage, without which
it'll be hard to share data between threads. But the attempt of using it is currently
unsuccessful. I'm still trying to figure out the correct way to use this derivitive. My guess now is that it
requires a specific regions of memory being marked correctly, and corresponding GDT entries should
be correct as well.

In a single core scenario, accessing process table (or ready queue) is trivial. But in a multicore
platform, Rust will not allow you to share the ready queue between threads without a
proper locking mechanism (one of the possible way is to wrap the entire queue with `Mutex`). 
But wrapping the entire process table
with a giant lock is not the way to go. I'm still figuring out a fine-grained locking scheme that
* Allow concurrent access to ready queue 
* Allow proper locking (no deadlocks or livelocks) before and after context switch (locks may
be held across context switch)
* No locks at places that does not require a lock

This part will be investigated in the summer.

## Threading
Here I use the simplest round robin scheduler to select
the next process to run.
