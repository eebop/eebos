#ifndef __MALLOC_H
#define __MALLOC_H

#include <stdint.h>

// Sets up malloc
void memsetup(void);

void terminal_initialize(void);

void *malloc(uint32_t size);

void *malloc_aligned(uint32_t size, uint32_t alignment);

// void *realloc(void *ptr, uint32_t size);

// void free(void *ptr);
void printf(char *str, ...);

#endif