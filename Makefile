CC = i686-elf-gcc
AS = i686-elf-as
NAS = nasm

CFLAGS = -std=gnu23 -ffreestanding -Wall -Wextra -O2

OBJECTS = boot.o kernel.o stdutils.o gdt.o pic.o ports.o irq.o mouse.o

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

kernel.o: stdutils.h
%.o: %.nasm
	nasm -f elf32 $< -o $@

clean:
	-rm *.o
	-rm eebos.iso
	-rm eebos.bin
	-rm isodir/boot/eebos.bin
	-rm isodir/boot/grub/grub.cfg