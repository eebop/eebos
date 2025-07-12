#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdarg.h>

#include "gdt.h"
#include "pic.h"
#include "ps2/controller.h"


/* Check if the compiler thinks you are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif

/* This tutorial will only work for the 32-bit ix86 targets. */
#if !defined(__i386__)
#error "This tutorial needs to be compiled with a ix86-elf compiler"
#endif

/* Hardware text mode color constants. */
enum vga_color
{
	VGA_COLOR_BLACK = 0,
	VGA_COLOR_BLUE = 1,
	VGA_COLOR_GREEN = 2,
	VGA_COLOR_CYAN = 3,
	VGA_COLOR_RED = 4,
	VGA_COLOR_MAGENTA = 5,
	VGA_COLOR_BROWN = 6,
	VGA_COLOR_LIGHT_GREY = 7,
	VGA_COLOR_DARK_GREY = 8,
	VGA_COLOR_LIGHT_BLUE = 9,
	VGA_COLOR_LIGHT_GREEN = 10,
	VGA_COLOR_LIGHT_CYAN = 11,
	VGA_COLOR_LIGHT_RED = 12,
	VGA_COLOR_LIGHT_MAGENTA = 13,
	VGA_COLOR_LIGHT_BROWN = 14,
	VGA_COLOR_WHITE = 15,
};

static inline uint8_t vga_entry_color(enum vga_color fg, enum vga_color bg)
{
	return fg | bg << 4;
}

static inline uint16_t vga_entry(unsigned char uc, uint8_t color)
{
	return (uint16_t)uc | (uint16_t)color << 8;
}

size_t strlen(const char *str)
{
	size_t len = 0;
	while (str[len])
		len++;
	return len;
}

static const size_t VGA_WIDTH = 80;
static const size_t VGA_HEIGHT = 25;

size_t terminal_row;
size_t terminal_column;
uint8_t terminal_color;
uint16_t *terminal_buffer;

void terminal_initialize(void)
{
	terminal_row = 0;
	terminal_column = 0;
	terminal_color = vga_entry_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
	terminal_buffer = (uint16_t *)0xB8000;
	for (size_t y = 0; y < VGA_HEIGHT; y++)
	{
		for (size_t x = 0; x < VGA_WIDTH; x++)
		{
			const size_t index = y * VGA_WIDTH + x;
			terminal_buffer[index] = vga_entry(' ', terminal_color);
		}
	}
}

void terminal_setcolor(uint8_t color)
{
	terminal_color = color;
}

void terminal_putentryat(char c, uint8_t color, size_t x, size_t y)
{
	const size_t index = y * VGA_WIDTH + x;
	terminal_buffer[index] = vga_entry(c, color);
}

void tputc(char c)
{
	if (c == '\n')
	{
		terminal_column = 0;
		terminal_row++;
	}
	else
	{
		terminal_putentryat(c, terminal_color, terminal_column, terminal_row);
		if (++terminal_column == VGA_WIDTH)
		{
			terminal_column = 0;
			terminal_row++;
		}
	}

	if (terminal_row == VGA_HEIGHT)
		terminal_row = 0;
}

void terminal_write(const char *data, size_t size)
{
	for (size_t i = 0; i < size; i++)
		tputc(data[i]);
}

void tputs(const char *data)
{
	terminal_write(data, strlen(data));
}

static char nums[] = "0123456789abcdef";

void tputd(uint32_t d, int base)
{
	if (d == 0) {
		tputc('0');
		return;
	}

	char buf[32] = {};
	tputs(buf);
	int index = 30;
	while (d)
	{
		buf[index--] = nums[d % base];
		d = d / base;
	}
	tputs(buf + index + 1);
}

void printf(char *str, ...) {
	va_list args;
	va_start(args);

	for (char c; c = *str; str++)
	{
		if (c == '%')
		{
			c = *++str;
			if (c == '%')
			{
				tputc('%');
			}
			else if (c == 'd')
			{
				tputd(va_arg(args, int), 10);
			}
			else if (c == 'x')
			{
				tputd(va_arg(args, int), 16);
			} else if (c == '?') {
				if (va_arg(args, int)) {
					tputs("True");
				} else {
					tputs("False");
				}
			} else if (c == 'b') {
				tputd(va_arg(args, int), 2);
			} else if (c == 'c') {
				tputc(va_arg(args, int));
			} else if (c == 's') {
				tputs(va_arg(args, char *));
			} else {
				tputs("<unknown indentifier>");
				if (c == '\0')
					return;
			}
		}
		else
		{
			tputc(c);
		}
	}
	va_end(args);
}

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
