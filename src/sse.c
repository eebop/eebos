#include "sse.h"

void initSSE(void) {
    uint32_t cr0;
    asm volatile (
        "mov %%cr0, %0"
        : "=g" (cr0)
    );
    cr0 &= ~(4);
    cr0 |= 2;
    asm volatile (
        "mov %0, %%cr0"
        :: "g" (cr0)
    );
    uint32_t cr4;
    asm volatile (
        "mov %%cr4, %0"
        : "=g" (cr4)
    );
    cr4 |= (1 << 9) + (1 << 10);
    asm volatile (
        "mov %0, %%cr4"
        :: "g" (cr4)
    );
}