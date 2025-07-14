#ifndef __GDT_H
#define __GDT_H

#include <stdint.h>

typedef struct {
    uint32_t limit;
    uint32_t base;
    uint8_t access;
    uint8_t flags;

} GDTEntry;

typedef struct {
    uint32_t offset;
    uint16_t segment;
    uint8_t gate;
    uint8_t privilege;
} IDTEntry;

void encodeGdtEntry(uint8_t *target, GDTEntry source);
void encodeIDTEntry(uint8_t *target, IDTEntry source);
void lgdt(uint8_t *target, uint16_t size);
void lidt(uint8_t *target, uint16_t size);

#endif