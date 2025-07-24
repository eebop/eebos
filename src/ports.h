#ifndef __PORTS_H
#define __PORTS_H
#include <stdint.h>

extern void outb(uint16_t port, uint8_t val);
extern uint8_t inb(uint16_t port);
extern void io_wait(void);

#endif