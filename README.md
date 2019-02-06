# ros_64

A toy kernel written in Rust (WIP), following [this blog](https://os.phil-opp.com/).

To build and run, type
```
make qemu
```
to enable gdb debugging, run
```
make qemu-gdb
```
The above two commands run with tiny bootloader located in src/bootloader.
To load the kernel using GRUB (CDROM), run
```
make qemu-iso
```
or
```
make qemu-iso-gdb
```

Boot without using bootimage crate.
The bootloader is a modified version of the tiny bootloader in 32bit
xv6 kernel.
