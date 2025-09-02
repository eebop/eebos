import lief
import sys

assert(len(sys.argv) == 3)

with open(sys.argv[1], 'rb') as f:
    elf = lief.parse(f)
    print(elf)
    for reloc in elf.relocations:
        if reloc == lief.ELF.Relocation.TYPE.X86_64_64:
            reloc = lief.ELF.Relocation.TYPE.X86_64_NONE            
