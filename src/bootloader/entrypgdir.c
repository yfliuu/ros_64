// #include "types.h"
// #include "mmu.h"
// #include "memlayout.h"

// __attribute__((__aligned__(PGSIZE)))
// pde_t entrypgdir[NPDENTRIES] = {
//   [0] = (0) | PTE_P | PTE_W | PTE_PS,
//   [KERNBASE>>PDXSHIFT] = (0) | PTE_P | PTE_W | PTE_PS,
// };
void _start() {
	volatile unsigned short *video = (volatile unsigned short*)0xb8000;
    video[640] = 0x769;

    while(1) {}
}