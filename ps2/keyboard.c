#include "keyboard.h"

static const char PS2SCS1_charset[] = "  1234567890-=\b\tqwertyuiop[]\n asdfghjkl;\'` \\zxcvbnm,./";

void keyboard_in(void) {
	uint8_t key = inb(0x60);
	char pressed = ' ';

	if ((key & 0x7f) <= 0x35) {
		pressed = PS2SCS1_charset[key & 0x7f];
	}

	printf("%s: %c (0x%x)\n", key & 0x80 ? "released" : "pressed", pressed, pressed);
	PIC_sendEOI(1);
}
