#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "gdt.h"
#include "pic.h"
#include "stdutils.h"
#include "page64.h"
#include "sse.h"
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

typedef struct {
	uint32_t interrupt;
	uint32_t edi;
	uint32_t esi;
	uint32_t ebp;
	uint32_t edx;
	uint32_t ecx;
	uint32_t ebx;
	uint32_t eax;
	uint32_t esp;
	uint32_t eip;
	uint32_t cs; // upper 16 bits must be 0 (must be u32 for alignment reasons)
	uint32_t eflags;
	// TODO: add mmx, etc
} regs;

void (*interrupts[256])(regs *) = {};

// void isr_handler(regs *r) {
// 	if (interrupts[r->interrupt] != 0) {
// 		interrupts[r->interrupt](r);
// 	}
// 	return;
// }

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
extern void rustmain(uint8_t *ptr);


extern uint8_t _binary_test_mod_start;
extern uint8_t _binary_test_mod_end;
extern uint8_t _binary_test_mod_size;

extern uint32_t stack_top;


void kernel_main(void)
{
	/* Initialize terminal interface */
	terminal_initialize();

	// printf("RO START: 0x%x, RW START: 0x%x, K END: 0x%x\n", KERNEL_START_RO, KERNEL_START_RW, KERNEL_END);

	// ((void (*)(void)) isr_table[44])();

	memsetup();

	uint8_t *gdtarray = malloc(0x28);

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

	initSSE();
	// reload_segs(0x8, 0x10);

	// for (int i = 0; i!=10;i++) {
	// 	printf("%x %x %x %x %x %x %x %x\n", isr_table[i * 8 + 0], isr_table[i * 8 + 1], isr_table[i * 8 + 2], isr_table[i * 8 + 3], isr_table[i * 8 + 4], isr_table[i * 8 + 5], isr_table[i * 8 + 6], isr_table[i * 8 + 7]);
	// }

	printf("TEST AFTER GDT\n");


	uint8_t *idtarray = malloc(256 * 8);

	IDTEntry entry;
	entry.privilege = 0;
	entry.segment = 0x08;
	entry.gate = 0xF;

	printf("out: %x\n", isr_table[0]);

	for (int i = 0; i != 256; i++)
	{
		entry.offset = (uint32_t) isr_table[i];
		encodeIDTEntry(&idtarray[8 * i], entry);
	}

	PIC_remap(0x20, 0x28);
	

	for (int x=0;x!=16;x++) {
			IRQ_set_mask(x);
	}

	IRQ_clear_mask(2);

	lidt(idtarray, 256 * 8 - 1);
	
	printf("Interrupts: %x\n", are_interrupts_enabled());

	// init_pages();

	printf("lgdt2\n");

    // asm volatile (
    //     ".global main64\n"
	// 	".global rustmain\n"
	// 	"xchgw %%bx, %%bx\n"


    //     "jmp $0x18, $main64\n"
	// 	".code64\n"
	// 	".global test64\n"
	// 	"main64:\n"
    //     // "xor %%rax, %%rax\n"
    //     // "mov $0xFFFFFFFFFFFFFFFF, %%rax\n"
    //     // "mov %%rax, 0xB8000\n"
	// 	"call rustmain\n"
	// 	"hlt\n"
    //     ".code32\n"
    //     :: [lptr] "m" (lptr)
    //     // ::  [ss] "g" (ss),
    //     //     [rsp] "g" (rsp),
    //     //     [eflags] "g" (eflags),
    //     //     [cs] "g" (cs)
    //         // [rip] "g" (rip)
    //     : "rax"
    // );

	int x = 10;
	int o = 5;

	rustmain(malloc(0));

	asm("push %[x]\n"
		"mov $3, %%ebx\n"
		"xchgw %%bx, %%bx\n"
		"int $0xff\n"
		"mov %%ebx, %[o]\n"
		"pop %[x]\n"
		: [x] "+r" (x),
		  [o] "=g" (o)
		:: "ebx"
	);

	printf("debug: %d = 10, %d (ebx) = 3", x, o);
	
	while (1) {
	}

}
