CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm
RSC = rustc

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -Og -g -fno-omit-frame-pointer
RSFLAGS = -O --crate-type=bin --emit=obj --target=i686-unknown-linux-gnu -C panic=abort -C lto=true -C code-model=small -C no-redzone=true

QEMUFLAGS = -no-reboot -no-shutdown -debugcon stdio #-d cpu_reset,int

# all filenames to build, minus extension
srcs = boot kernel stdutils gdt pic ports irq page64 core64 sse

modules = test_mod test_dep paging process interrupts pic dyshared#start_process pic

libmod = shared

builddir = $(abspath build)

srcdir = src

OBJECTS = $(addprefix $(builddir)/,$(srcs:%=%.o))

HEADERS = $(foreach f,$(srcs:%=%.h),$(wildcard src/$f))

CSRC = $(foreach f,$(srcs:%=%.c),$(wildcard src/$f))

RSRC = $(foreach f,$(srcs),$(wildcard $f/Cargo.toml))

# OBJMODS = $(addprefix $(builddir)/mods/,$(modules))

kqemu: build/eebos.bin
	qemu-system-i386 -kernel build/eebos.bin $(QEMUFLAGS)

gdb: build/eebos.bin
	qemu-system-i386 -kernel build/eebos.bin $(QEMUFLAGS) -S -s -serial tcp::4444,server=on

qemu: build/eebos.iso
	qemu-system-i386 -cdrom build/eebos.iso $(QEMUFLAGS)

bochs: build/eebos.iso
	bochs -debugger

include makefile.deps

build: build/eebos.iso

build/eebos.iso: build/isodir/boot/grub/grub.cfg build/isodir/boot/eebos.bin
	grub-mkrescue -o $@ build/isodir

build/isodir/boot/grub/grub.cfg: grub.cfg
	@mkdir -p build/isodir/boot/grub
	cp $< $@

build/isodir/boot/eebos.bin: build/eebos.bin
	@mkdir -p build/isodir/boot
	cp $< $@

build/eebos.bin: linker.ld $(OBJECTS)
	i686-elf-gcc -T linker.ld -o $@ -ffreestanding -Og -nostdlib $(OBJECTS) -z noexecstack -Wl,--gc-sections -Wl,--demangle -g


$(builddir)/%.o: $(srcdir)/%.nasm
	@mkdir -p $(dir $@)
	$(NAS) -f elf32 $< -o $@

$(builddir)/%.o: $(RSCMP) $(srcdir)/%.rs
	echo $(RSCMP)
	@mkdir -p $(dir $@)
	$(RSC) $(RSFLAGS) $(srcdir)/$*.rs -o $@

$(builddir)/%.o: $(srcdir)/%.s
	@mkdir -p $(dir $@)
	$(AS) $(ASFLAGS) $< -o $@

$(builddir)/%.o: $(srcdir)/%.c
	mkdir -p $(dir $@)
	$(CC) $(CPPFLAGS) $(CFLAGS) $< -c -o $@

core64/target/target/debug/libcore64.a: core64/src/*.rs $(libmod)/src/*.rs $(builddir)/mods/test_mod
	cd core64 ; cargo rustc -Zjson-target-spec --target=target.json -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem -- -Crelocation-model=static


$(builddir)/core64.o: core64/target/target/debug/libcore64.a
	@mkdir -p build
	cp -f core64/target/target/debug/libcore64.a $@

$(builddir)/mods/passthrough/libpassthrough.so: modules/passthrough/passthrough.c
	@mkdir -p $(builddir)/mods/passthrough
	clang $< -shared -fPIC -o $@ -nostdlib --target=i386-unknown-none


$(builddir)/mods/test_mod: $(builddir)/mods/passthrough/libpassthrough.so modules/test_mod/src/main.rs modules/*/src/*.rs
	@mkdir -p build/mods
	cd modules/test_mod; RUSTFLAGS="-Cno-redzone=true -Crelocation-model=pie -L$(builddir)/mods/passthrough -l passthrough" cargo rustc --target=i686-unknown-linux-gnu -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem --release -- -Clink-args="-nostdlib -Wl,--no-dynamic-linker"
	cp modules/test_mod/target/i686-unknown-linux-gnu/release/test_mod $@


# $(builddir)/mods/%: $(builddir)/mods/%.so
# 	i686-elf-objcopy -I binary -O elf32-i386 --binary-symbol-prefix=_binary_$* $< $@ 

makefile.deps: $(HEADERS) $(CSRC)
	$(CC) -MM $(CSRC) > makefile.deps

clean:
	-rm -rf build
	-rm makefile.deps
	for i in $(RSRC:%/Cargo.toml=%) modules/*; do \
		pwd=$$(pwd); \
		cd $$i; cargo clean ; cd $$pwd; \
	done;
	cd core64; cargo clean
	-rm core64/target/target/debug/libcore64.rlib
