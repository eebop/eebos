#include <stdint.h>
#include <stddef.h>
#include <stdarg.h>

#include "stdutils.h"

extern unsigned char KERNEL_END;

uint8_t *memptr;

void memsetup(void) {
    memptr = &KERNEL_END;
}

void *malloc(uint32_t size) {
    uint32_t out = (uint32_t) memptr;
    memptr = memptr + size;
	return (void *) out;
}

void *malloc_aligned(uint32_t size, uint32_t alignment) {
	malloc((alignment - (((uint32_t) memptr) & (alignment - 1))) % alignment);
	return malloc(size);
}

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
	return (uint16_t) uc | (uint16_t)color << 8;
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
		while (terminal_column != VGA_WIDTH)
		{
			terminal_putentryat(' ', terminal_color, terminal_column, terminal_row);
			terminal_column++;

		}
		
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

	char buf[33] = {};
	tputs(buf);
	int index = 31;
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
