CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm
RSC = rustc

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -O2
RSFLAGS = -O --crate-type=bin --emit=obj --target=i686-unknown-linux-gnu -C panic=abort -C lto=true -C code-model=small -C no-redzone=true

# all filenames to build, minus extension
srcs = boot kernel stdutils gdt pic ports irq ps2/control #ps2/controller ps2/mouse ps2/keyboard 

builddir = build

srcdir = src

OBJECTS = $(addprefix $(builddir)/,$(srcs:%=%.o))

HEADERS = $(foreach f,$(srcs:%=%.h),$(wildcard src/$f))

CSRC = $(foreach f,$(srcs:%=%.c),$(wildcard src/$f))

RSCMP = $(foreach f,$(srcs:%=%_h.rs),$(wildcard src/$f))

kqemu: build/eebos.bin
	qemu-system-i386 -kernel build/eebos.bin

qemu: build/eebos.iso
	qemu-system-i386 -cdrom build/eebos.iso

bochs: build/eebos.iso
	bochs

build/eebos.iso: build/isodir/boot/grub/grub.cfg build/isodir/boot/eebos.bin
	grub-mkrescue -o $@ build/isodir

build/isodir/boot/grub/grub.cfg: build/grub.cfg
	mkdir -p build/isodir/boot/grub
	cp $< $@

build/isodir/boot/eebos.bin: build/eebos.bin
	mkdir -p build/isodir/boot
	cp $< $@

build/eebos.bin: linker.ld ${OBJECTS}
	${CC} -T linker.ld -o $@ -ffreestanding -O2 -nostdlib ${OBJECTS} -lgcc -z noexecstack

$(builddir)/%.o: $(srcdir)/%.nasm
	mkdir -p $(dir $@)
	$(NAS) -f elf32 $< -o $@

$(builddir)/%.o: $(RSCMP) $(srcdir)/%.rs
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
	bindgen --use-core --ctypes-prefix=cty $< -o $@

makefile.deps: $(HEADERS) $(CSRC)
	for file in $(patsubst %.rs,%_rs.h,$(foreach f,$(srcs:%=%.rs),$(wildcard src/$f))); do \
		if test ! -f $$file; then \
			touch -t 197001010101 $$file; \
		fi; \
	done
	$(CC) -MM $(CSRC) > makefile.deps

include makefile.deps

clean:
	-rm -rf build
	-rm makefile.deps
	-rm $(foreach f,$(srcs:%=%_h.rs),$(wildcard src/$f))
	-rm $(foreach f,$(srcs:%=%_rs.h),$(wildcard src/$f))
