#include "controller.h"
#include "keyboard.h"
#include "mouse.h"

void ps2_write(PS2PORT port, uint8_t value) {
    int max_tries = 1000;
    uint8_t lock;
    do {
        lock = inb(0x64);
    } while ((lock & 0x2) && --max_tries);
    if (!max_tries) {
        // kerror()
    }
    return outb(port, value);
}

uint8_t ps2_read(PS2PORT port) {
    int max_tries = 1000;
    uint8_t lock;
    do {
        lock = inb(0x64);
    } while (!(lock & 0x1) && --max_tries);
    if (!max_tries) {
        // kerror()
    }
    return inb(port);
}

void ps2_device_write(PS2TOKEN token, uint8_t value) {
    if (token == AUX) {
        ps2_write(CMD, 0xD4);
    }
    ps2_write(DATA, value);
}

void ps2_init(void) {
    ps2_write(CMD, 0x20);
    uint8_t ccb = ps2_read(DATA);

    ccb = ccb | 3;
    ccb = ccb & ~((1 << 6) + (1 << 5));

    ps2_write(CMD, 0x60);
    ps2_write(DATA, ccb);

    mouse_init(AUX);

    ps2_write(CMD, 0xA8);

    IRQ_clear_mask(1);
    IRQ_clear_mask(12);

}