#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "gdt.h"
#include "pic.h"
#include "stdutils.h"
#include "page64.h"
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

extern void main64(void);

extern uint32_t stack_top;


void kernel_main(void)
{
	/* Initialize terminal interface */
	terminal_initialize();

	// printf("RO START: 0x%x, RW START: 0x%x, K END: 0x%x\n", KERNEL_START_RO, KERNEL_START_RW, KERNEL_END);

	// ((void (*)(void)) isr_table[44])();

	memsetup();

	uint8_t gdtarray[0x28];//= malloc(0x28);

	GDTSegment nulle = {};

	GDTSegment kcode32 = {
		.limit = 0xFFFFF,
		.accessed = 1,
		.readwrite = 1,
		.direct_conform = 0,
		.exec = 1,
		.phys = 1,
		.privilege = 0,
		.granularity = 1,
		.is32 = 1,
		.is64 = 0
	};

	GDTSegment kdata32 = {
		.limit = 0xFFFFF,
		.accessed = 1,
		.readwrite = 1,
		.direct_conform = 0,
		.exec = 0,
		.phys = 1,
		.privilege = 0,
		.granularity = 1,
		.is32 = 1,
		.is64 = 0
	};

	GDTSegment kcode64 = {
		.limit = 0xFFFFF,
		.accessed = 1,
		.readwrite = 1,
		.direct_conform = 0,
		.exec = 1,
		.phys = 1,
		.privilege = 0,
		.granularity = 1,
		.is32 = 0,
		.is64 = 1
	};

	GDTSegment kdata64 = {
		.limit = 0xFFFFF,
		.accessed = 1,
		.readwrite = 1,
		.direct_conform = 0,
		.exec = 0,
		.phys = 1,
		.privilege = 0,
		.granularity = 1,
		.is32 = 0,
		.is64 = 1
	};


	encodeGDTEntry32(&gdtarray[0x00], nulle, 0);
	encodeGDTEntry32(&gdtarray[0x08], kcode32, 0);
	encodeGDTEntry32(&gdtarray[0x10], kdata32, 0);
	encodeGDTEntry32(&gdtarray[0x18], kcode64, 0);
	encodeGDTEntry32(&gdtarray[0x20], kdata64, 0);

	lgdt(gdtarray, 0x27);

	asm volatile (
		"xchgw %bx, %bx\n"
	);

	// reload_segs(0x8, 0x10);

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

	// ps2_init();

	init_pages();


	// encodeGDTEntry32(&gdtarray[0x08], kcode64, 0);
	// encodeGDTEntry32(&gdtarray[0x10], kdata64, 0);
	// lgdt(gdtarray, 23);

	printf("lgdt2\n");

	// asm volatile (
	// 	"xchgw %bx, %bx\n"
	// );
 
	uint32_t eflags;
    asm (
        "pushf\n"
        "pop %0"
        : "=g" (eflags)
    );
    uint32_t cs = 0x18;
    uint32_t rsp = stack_top;
    uint32_t ss = 0x20;

    struct fp {
        uint32_t offset;
        uint16_t segment;
    } __attribute__((packed));
    struct fp lptr;
    lptr.segment = 0x18;
    lptr.offset = (uint32_t) main64;



    asm volatile (
        ".global main64\n"
        // "push %[ss64]\n"
        // // "push $0\n"
        // "push %[rsp]\n"
        // // "push $0\n"
        // "push %[rflags]\n"
        // // "push $0\n"
        // "push %[cs64]\n"
        // // "push $0\n"
        // "push main64\n"
        // // "push $0\n"
        // "iret\n"
		// "xchgw %%bx, %%bx\n"
        // "mov $0x10, %%ax\n"
        // "mov %%ax, %%ds\n"
        // "mov %%ax, %%es\n"
        // "mov %%ax, %%fs\n"
        // "mov %%ax, %%gs\n"
        // "mov %%ax, %%ss\n"
		// "push $0x08\n"
		// "lea %%rax, .reload_CS(%%rip)\n"
		// "push %%rax\n"
		// "lretq\n"
		"xchgw %%bx, %%bx\n"


        "jmp $0x18, $main64\n"
        ".code64\n"
        "main64:\n"
        "xor %%rax, %%rax\n"
        "mov $0xFFFFFFFFFFFFFFFF, %%rax\n"
        "mov %%rax, 0xB8000\n"
		"hlt\n"
        ".code32\n"
        :: [lptr] "m" (lptr)
        // ::  [ss] "g" (ss),
        //     [rsp] "g" (rsp),
        //     [eflags] "g" (eflags),
        //     [cs] "g" (cs)
            // [rip] "g" (rip)
        : "rax"
    );

	printf("here in 32-bit mode\n");

	while (1) {
	}

}
