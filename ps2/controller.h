#ifndef __PS2_CONTROLLER_H
#define __PS2_CONTROLLER_H
#include "../ports.h"
#include "../pic.h"
#include "../stdutils.h"

typedef enum {
    MAIN = 1,
    AUX = 2
} PS2TOKEN;

typedef enum {
    CMD = 0x64,
    DATA = 0x60
} PS2PORT;

void ps2_init();
void ps2_device_write(PS2TOKEN token, uint8_t value);
uint8_t ps2_read(PS2PORT port);

typedef union {
    keyboard_state *keyboard,
    mouse_state *mouse
} state;

#endif