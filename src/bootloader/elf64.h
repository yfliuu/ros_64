// Format of an ELF executable file

#define ELF_MAGIC 0x464C457FU  // "\x7FELF" in little endian

struct elf64hdr {
  uint magic;  // must equal ELF_MAGIC
  uchar elf[12];
  ushort type;
  ushort machine;
  uint version;
  uint entry;
  uint entry_pad;
  uint phoff;
  uint phoff_pad;
  uint shoff;
  uint shoff_pad;
  uint flags;
  ushort ehsize;
  ushort phentsize;
  ushort phnum;
  ushort shentsize;
  ushort shnum;
  ushort shstrndx;
};

// Program section header
struct prog64hdr {
  uint type;
  uint flags;
  uint off;
  uint off_pad;
  uint vaddr;
  uint vaddr_pad;
  uint paddr;
  uint paddr_pad;
  uint filesz;
  uint filesz_pad;
  uint memsz;
  uint memsz_pad;
  uint align;
  uint align_pad;
};
