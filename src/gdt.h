#ifndef __GDT_H
#define __GDT_H

#include <stdint.h>
#include "stdutils.h"


typedef struct {
    uint32_t limit;
    // flags
    uint8_t granularity : 1;
    uint8_t is32 : 1;
    uint8_t is64 : 1;
    // access
    uint8_t privilege : 2;
    uint8_t phys : 1;
    uint8_t exec : 1;
    uint8_t direct_conform : 1;
    uint8_t readwrite : 1;
    uint8_t accessed : 1;
} GDTSegment;

typedef struct {
    uint32_t offset;
    uint16_t segment;
    uint8_t gate;
    uint8_t privilege;
} IDTEntry;

void encodeGDTEntry32(uint8_t *target, GDTSegment source, uint32_t base);
void encodeGDTEntry64(uint8_t *target, GDTSegment source, uint64_t base);
void encodeIDTEntry(uint8_t *target, IDTEntry source);
void lgdt(uint8_t *target, uint16_t size);
void reload_segs(uint16_t code, uint16_t data);
void lidt(uint8_t *target, uint16_t size);

#endif