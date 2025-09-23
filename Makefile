CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm
RSC = rustc

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -O2
RSFLAGS = -O --crate-type=bin --emit=obj --target=i686-unknown-linux-gnu -C panic=abort -C lto=true -C code-model=small -C no-redzone=true

QEMUFLAGS = -no-reboot -no-shutdown #-d cpu_reset,int

# all filenames to build, minus extension
srcs = boot kernel stdutils gdt pic ports irq page64 core64 sse

modules = test_mod pic

builddir = build

srcdir = src

OBJECTS = $(addprefix $(builddir)/,$(srcs:%=%.o))

HEADERS = $(foreach f,$(srcs:%=%.h),$(wildcard src/$f))

CSRC = $(foreach f,$(srcs:%=%.c),$(wildcard src/$f))

RSRC = $(foreach f,$(srcs),$(wildcard $f/Cargo.toml))

OBJMODS = $(addprefix $(builddir)/mods/,$(modules:%=%.o))

kqemu: build/eebos.bin
	qemu-system-x86_64 -kernel build/eebos.bin $(QEMUFLAGS)

qemu: build/eebos.iso
	qemu-system-x86_64 -cdrom build/eebos.iso $(QEMUFLAGS)

bochs: build/eebos.iso
	bochs -debugger

build: build/eebos.iso

build/eebos.iso: build/isodir/boot/grub/grub.cfg build/isodir/boot/eebos.bin
	grub-mkrescue -o $@ build/isodir

build/isodir/boot/grub/grub.cfg: grub.cfg
	mkdir -p build/isodir/boot/grub
	cp $< $@

build/isodir/boot/eebos.bin: build/eebos.bin
	mkdir -p build/isodir/boot
	cp $< $@

build/eebos.bin: linker.ld $(OBJECTS) $(OBJMODS)
	i686-elf-gcc -T linker.ld -o $@ -ffreestanding -O2 -nostdlib $(OBJECTS) $(OBJMODS) -z noexecstack -Wl,--gc-sections -Wl,--demangle

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

$(builddir)/core64.o: core64/src/*.rs
	mkdir -p build
	cd core64 ; cargo rustc --release --target=target.json -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem -- --emit=obj
	cd core64/target/target/release/deps; for i in *.rlib; do \
		mkdir -p $${i%.rlib}; cd $${i%.rlib}; ar x ../$$i; \
		ar r ../../libcore64.rlib *; \
		cd ..; \
	done
	cp core64/target/target/release/libcore64.rlib $@



$(builddir)/mods/%.o: modules/%/src/main.rs modules/%/src/*.rs
	mkdir -p build/mods
	cd modules/$* ; cargo rustc --release --target=i686-unknown-linux-gnu -- -Ctarget-feature=+crt-static -Crelocation-model=pie
	cp modules/$*/target/i686-unknown-linux-gnu/release/$* $*
	i686-elf-objcopy -I binary -O elf32-i386 $* $@
	mv $* $(builddir)/mods/$*

makefile.deps: $(HEADERS) $(CSRC)
	$(CC) -MM $(CSRC) > makefile.deps

include makefile.deps

clean:
	-rm -rf build
	-rm makefile.deps
	for i in $(RSRC:%/Cargo.toml=%) modules/*; do \
		pwd=$$(pwd); \
		cd $$i; cargo clean -p $$( basename $$i ) ; cd $$pwd; \
	done;
