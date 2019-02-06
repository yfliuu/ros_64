ARCH := amd64

TOOLPREFIX :=

# Toolchain commands
CC := $(TOOLPREFIX)gcc
RUSTC := rustc
AS := $(TOOLPREFIX)as
LD := $(TOOLPREFIX)ld
OBJCOPY := $(TOOLPREFIX)objcopy
OBJDUMP := $(TOOLPREFIX)objdump

# Object directory
OBJDIR := target/x86_64-ros/debug/

# Compiler options (FOR BOOTLOADER ONLY)
CFLAGS := -fno-pic -static -fno-builtin -fno-strict-aliasing -O2 -Wall -MD -ggdb -m32 -Werror -fno-omit-frame-pointer
CFLAGS += $(shell $(CC) -fno-stack-protector -E -x c /dev/null >/dev/null 2>&1 && echo -fno-stack-protector)
ASFLAGS := -m32 -gdwarf-2 -Wa,-divide
LDFLAGS := -m elf_i386

# Objects (We only build bootloader using gcc toolchain, leave the rest to cargo)
BTLDERDIR := src/bootloader/
BIN := ros.img

# CDROM booting.
ISOBIN := ros.iso
ISODIR := target/isodir/

# Debug/Emulation options
CPUS := 2
GDBPORT := $(shell expr `id -u` % 5000 + 25000)
QEMU := qemu-system-x86_64
# QEMUOPTS := -kernel $(OBJDIR)ros -smp $(CPUS) -m 512
QEMUCOMMON := -drive file=$(OBJDIR)fs.img,index=1,media=disk,format=raw \
			  -smp $(CPUS) -m 512 -monitor stdio
QEMUOPTS := $(QEMUCOMMON) -drive file=$(BIN),index=0,media=disk,format=raw
QEMUGDB := $(shell if $(QEMU) -help | grep -q '^-gdb'; \
           	then echo "-gdb tcp::$(GDBPORT)"; \
           	else echo "-s -p $(GDBPORT)"; fi)

# Simple bootloader.
$(OBJDIR)bootblock: $(BTLDERDIR)bootasm.S $(BTLDERDIR)bootmain.c
	$(CC) $(CFLAGS) -fno-pic -O -nostdinc -I. -c $(BTLDERDIR)bootmain.c -o $(OBJDIR)bootmain.o
	$(CC) $(CFLAGS) -fno-pic -nostdinc -I. -c $(BTLDERDIR)bootasm.S -o $(OBJDIR)bootasm.o
	$(LD) $(LDFLAGS) -N -e start -Ttext 0x7C00 -o $(OBJDIR)bootblock.o $(OBJDIR)bootasm.o $(OBJDIR)bootmain.o
	$(OBJDUMP) -S $(OBJDIR)bootblock.o > $(OBJDIR)bootblock.asm
	$(OBJCOPY) -S -O binary -j .text $(OBJDIR)bootblock.o $(OBJDIR)bootblock
	./sign.pl $(OBJDIR)bootblock

# This is just a dummy.
$(OBJDIR)fs.img:
	dd if=/dev/zero of=$(OBJDIR)fs.img count=1000

# Create a file, put bootblock in front and append ros
$(BIN): $(OBJDIR)ros $(OBJDIR)bootblock $(OBJDIR)fs.img
	dd if=/dev/zero of=$(BIN) count=10000
	dd if=$(OBJDIR)bootblock of=$(BIN) conv=notrunc
	dd if=$(OBJDIR)ros of=$(BIN) seek=1 conv=notrunc

# The binary built by cargo (ros) should be linked with entry stub according to linker script.
# The compilation of the entry stub (entry.S) is done in build script (build.rs).
$(OBJDIR)ros:
	cargo xbuild --target=x86_64-ros.json
	$(OBJDUMP) $(OBJDIR)ros -S > ros.asm

# CDROM booting.
$(ISODIR) $(ISODIR)boot $(ISODIR)boot/grub:
	mkdir -p $@

$(ISODIR)boot/ros: $(OBJDIR)ros $(ISODIR)boot
	cp $< $@

$(ISODIR)boot/grub/grub.cfg: grub.cfg $(ISODIR)boot/grub
	cp $< $@

$(ISOBIN): $(ISODIR)boot/ros $(ISODIR)boot/grub/grub.cfg
	grub-mkrescue -o $@ $(ISODIR)

.PHONY: check_multiboot

# Emulation. Remove check_multiboot qemu dependencies if not needed.
qemu: $(BIN) $(OBJDIR)fs.img check_multiboot
	$(QEMU) $(QEMUOPTS)

qemu-gdb: $(BIN) $(OBJDIR)fs.img .gdbinit check_multiboot
	@echo "*** Now run 'gdb'." 1>&2
	$(QEMU) $(QEMUOPTS) -S $(QEMUGDB)

qemu-iso: $(ISOBIN) $(OBJDIR)fs.img check_multiboot
	$(QEMU) -cdrom $(ISOBIN) $(QEMUCOMMON)

qemu-iso-gdb: $(ISOBIN) $(OBJDIR)fs.img .gdbinit check_multiboot
	@echo "*** Now run 'gdb'." 1>&2
	$(QEMU) -cdrom $(ISOBIN) $(QEMUCOMMON) -S $(QEMUGDB)

.gdbinit: .gdbinit.tmpl
	sed "s/localhost:1234/localhost:$(GDBPORT)/" < $^ > $@

# This command checks if the generated image is multiboot compatible.
check_multiboot: $(OBJDIR)ros
	@echo ""
	@echo "This command checks if the generated image is multiboot compatible."
	@echo "Remove the dependency from qemu & qemu-gdb if you write your own bootloader."
	@echo ""
	@command -v grub-file >/dev/null 2>&1 || { echo >&2 "grub-file command not found. Aborting."; exit 1; }
	@grub-file --is-x86-multiboot $(OBJDIR)ros || { echo >&2 "Image file is NOT multiboot compatible. Aborting."; exit 1; }
	@echo "Yay! It is compatible!"

# Cleanup
# No recompilation of core and builtins
clean:
	rm -f .gdbinit $(OBJDIR)ros \
		  $(OBJDIR)entry.o $(OBJDIR)bootblock $(BIN) ros.asm ros.iso
	rm -rf target/isodir

cclean:
	cargo clean
	rm -f .gdbinit ros.iso
