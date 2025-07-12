#ifndef __MALLOC_H
#define __MALLOC_H

#include <stdint.h>

// Sets up malloc
void memsetup(void);

void *malloc(uint32_t size);

// void *realloc(void *ptr, uint32_t size);

// void free(void *ptr);
void printf(char *str, ...);

#endif