CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -O2

OBJECTS = boot.o kernel.o stdutils.o gdt.o pic.o ports.o irq.o ps2/controller.o ps2/mouse.o ps2/keyboard.o

kqemu: eebos.bin
	qemu-system-i386 -kernel eebos.bin

qemu: eebos.iso
	qemu-system-i386 -cdrom eebos.iso

bochs: eebos.iso
	bochs

eebos.iso: isodir/boot/grub/grub.cfg isodir/boot/eebos.bin
	grub-mkrescue -o $@ isodir

isodir/boot/grub/grub.cfg: grub.cfg
	cp $< $@

isodir/boot/eebos.bin: eebos.bin
	cp $< $@

eebos.bin: linker.ld ${OBJECTS}
	${CC} -T linker.ld -o $@ -ffreestanding -O2 -nostdlib ${OBJECTS} -lgcc

%.o: %.nasm
	nasm -f elf32 $< -o $@

%.c:
	makefile.deps

makefile.deps:
	CC -MM *.c */*.c > makefile.deps

include makefile.deps

clean:
	-rm *.o
	-rm */*.o
	-rm eebos.iso
	-rm eebos.bin
	-rm isodir/boot/eebos.bin
	-rm isodir/boot/grub/grub.cfg