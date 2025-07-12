#include "mouse.h"

void mouse_init(PS2TOKEN token) {
    ps2_device_write(token, 0xF6);
    ps2_read(0x60);
    ps2_device_write(token, 0xF4);
    ps2_read(0x60);
    mouse_state *m = malloc(sizeof(mouse_state));
}

void mouse_in(void) {
	uint8_t var = inb(0x60);
	printf("here! %x\n", var);
	PIC_sendEOI(12);
}
