#include <stdint.h>

extern char KERNEL_END;

uint8_t *memptr;

void memsetup(void) {
    memptr = &KERNEL_END;
}

void *malloc(uint32_t size) {
    uint32_t out = (uint32_t) memptr;
    memptr = memptr + size;
    return (void *) out;
}