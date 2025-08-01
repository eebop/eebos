CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm
RSC = rustc

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -O2
RSFLAGS = -O --crate-type=bin --emit=obj --target=i686-unknown-linux-gnu -C panic=abort -C lto=true -C code-model=small -C no-redzone=true

QEMUFLAGS = -no-reboot -d cpu_reset,int -no-shutdown

# all filenames to build, minus extension
srcs = boot kernel stdutils gdt pic ports irq ps2/control page64 #ps2/controller ps2/mouse ps2/keyboard 

builddir = build

srcdir = src

OBJECTS = $(addprefix $(builddir)/,$(srcs:%=%.o))

HEADERS = $(foreach f,$(srcs:%=%.h),$(wildcard src/$f))

CSRC = $(foreach f,$(srcs:%=%.c),$(wildcard src/$f))

RSRC = $(foreach f,$(srcs:%=%.rs),$(wildcard src/$f))

RSCMP = $(HEADERS:%.h=%_h.rs)

CSCMP = $(RSRc:%.rs=%_rs.h)

kqemu: build/eebos.bin
	qemu-system-x86_64 -kernel build/eebos.bin $(QEMUFLAGS)

qemu: build/eebos.iso
	qemu-system-x86_64 -cdrom build/eebos.iso $(QEMUFLAGS)

bochs: build/eebos.iso
	bochs -debugger

build/eebos.iso: build/isodir/boot/grub/grub.cfg build/isodir/boot/eebos.bin
	grub-mkrescue -o $@ build/isodir

build/isodir/boot/grub/grub.cfg: grub.cfg
	mkdir -p build/isodir/boot/grub
	cp $< $@

build/isodir/boot/eebos.bin: build/eebos.bin
	mkdir -p build/isodir/boot
	cp $< $@

build/eebos.bin: linker.ld ${OBJECTS}
	$(CC) -T linker.ld -o $@ -ffreestanding -O2 -nostdlib ${OBJECTS} -lgcc -z noexecstack
$(builddir)/%.o: $(srcdir)/%.nasm
	mkdir -p $(dir $@)
	$(NAS) -f elf32 $< -o $@

$(builddir)/%.o: $(RSCMP) $(srcdir)/%.rs
	echo $(RSCMP)
	mkdir -p $(dir $@)
	$(RSC) $(RSFLAGS) $(srcdir)/$*.rs -o $@

$(builddir)/%.o: $(srcdir)/%.s
	mkdir -p $(dir $@)
	$(AS) $(ASFLAGS) $< -o $@

$(builddir)/%.o: $(srcdir)/%.c
	mkdir -p $(dir $@)
	$(CC) $(CPPFLAGS) $(CFLAGS) $< -c -o $@

%_rs.h: %.rs
	cbindgen -c cbindgen.toml $< -o $@

%_h.rs: %.h
	bindgen --use-core --block-extern-crate $< -o $@ \
	--raw-line '#![allow(dead_code)]' \
	--raw-line '#![allow(non_camel_case_types)]' \
	--raw-line '#![allow(non_upper_case_globals)]' \
	-- --target=i686-unknown-none

makefile.deps: $(HEADERS) $(CSRC) $(RSCMP) $(CSCMP)
	$(CC) -MM $(CSRC) > makefile.deps

include makefile.deps

clean:
	-rm -rf build
	-rm makefile.deps
	-rm $(RSCMP)
	-rm $(CSCMP)
