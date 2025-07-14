#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "gdt.h"
#include "pic.h"
#include "stdutils.h"
// #include "ps2/controller.h"


#include "ps2/control_rs.h"

/* Check if the compiler thinks you are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif

/* This tutorial will only work for the 32-bit ix86 targets. */
#if !defined(__i386__)
#error "This tutorial needs to be compiled with a ix86-elf compiler"
#endif

extern uint8_t KERNEL_START_RO[];

extern uint8_t KERNEL_START_RW[];

extern uint8_t KERNEL_END[];

void badexcept(void)
{
	printf("test interupt caught\n");
	PIC_sendEOI(8);
	while (1) {}
}

static inline bool are_interrupts_enabled()
{
    unsigned long flags;
    asm volatile ( "pushf\n\t"
                   "pop %0"
                   : "=g"(flags) );
    return flags & (1 << 9);
}

extern uint32_t isr_table[];
extern void isr_stub_33(void);

void io_wait2(void) {
	for (int i =0; i!= 100; i++) {
		io_wait();
	}
}

void kernel_main(void)
{
	/* Initialize terminal interface */
	terminal_initialize();

	// printf("RO START: 0x%x, RW START: 0x%x, K END: 0x%x\n", KERNEL_START_RO, KERNEL_START_RW, KERNEL_END);

	// ((void (*)(void)) isr_table[44])();

	memsetup();

	GDTEntry nulle;
	nulle.base = 0;
	nulle.limit = 0;
	nulle.access = 0;
	nulle.flags = 0;

	GDTEntry kcode;
	kcode.base = 0;
	kcode.limit = 0xFFFFF;
	kcode.access = 0x9a;
	kcode.flags = 0xC;

	GDTEntry kdata;
	kdata.base = 0;
	kdata.limit = 0xFFFFF;
	kdata.access = 0x92;
	kdata.flags = 0xC;

	encodeGdtEntry(&KERNEL_END[0x00], nulle);
	encodeGdtEntry(&KERNEL_END[0x08], kcode);
	encodeGdtEntry(&KERNEL_END[0x10], kdata);

	lgdt(KERNEL_END, 23);

	// for (int i = 0; i!=10;i++) {
	// 	printf("%x %x %x %x %x %x %x %x\n", isr_table[i * 8 + 0], isr_table[i * 8 + 1], isr_table[i * 8 + 2], isr_table[i * 8 + 3], isr_table[i * 8 + 4], isr_table[i * 8 + 5], isr_table[i * 8 + 6], isr_table[i * 8 + 7]);
	// }

	printf("TEST AFTER GDT\n");

	IDTEntry entry;
	entry.privilege = 0;
	entry.segment = 0x08;
	entry.gate = 0xF;

	for (int i = 0; i != 256; i++)
	{
		entry.offset = (uint32_t) isr_table[i];
		encodeIDTEntry(&KERNEL_END[0x18 + 8 * i], entry);
	}

	PIC_remap(0x20, 0x28);

	for (int x=0;x!=16;x++) {
			IRQ_set_mask(x);
	}

	IRQ_clear_mask(2);

	lidt(&KERNEL_END[0x18], 256 * 8 - 1);
	
	printf("Interrupts: %x\n", are_interrupts_enabled());

	ps2_init();

	while (1) {
	}

}
