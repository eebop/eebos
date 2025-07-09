#include "gdt.h"

typedef struct __attribute__((packed)) {
    uint16_t limit;
    uint32_t base;
} dtr;


void encodeGdtEntry(uint8_t *target, GDTEntry source)
{
    // Check the limit to make sure that it can be encoded
    if (source.limit > 0xFFFFF) {
        // printf("GDT cannot encode limits larger than 0xFFFFF");
        return;
    }
    
    // Encode the limit
    target[0] = source.limit & 0xFF;
    target[1] = (source.limit >> 8) & 0xFF;
    target[6] = (source.limit >> 16) & 0x0F;
    
    // Encode the base
    target[2] = source.base & 0xFF;
    target[3] = (source.base >> 8) & 0xFF;
    target[4] = (source.base >> 16) & 0xFF;
    target[7] = (source.base >> 24) & 0xFF;
    
    // Encode the access byte
    target[5] = source.access;
    
    // Encode the flags
    target[6] |= (source.flags << 4);
}

void encodeIDTEntry(uint8_t *target, IDTEntry source) {
    target[0] = (source.offset >> 0x00) & 0xFF;
    target[1] = (source.offset >> 0x08) & 0xFF;
    target[6] = (source.offset >> 0x10) & 0xFF;
    target[7] = (source.offset >> 0x18) & 0xFF;

    target[2] = (source.segment >> 0x00) & 0xFF;
    target[3] = (source.segment >> 0x08) & 0xFF;

    target[4] = 0;

    target[5] = 0;
    target[5] |= source.gate & 0xF;
    target[5] |= (source.privilege & 0x3) << 4;
    target[5] |= 1 << 7;
}

extern void reload_cs(void);

void lgdt(uint8_t *target, uint16_t size) {
    dtr gdtr;

    gdtr.limit = size;
    gdtr.base  = (uint32_t)target;

    struct FarPointer {
        uint32_t offset;
        uint16_t segment;
    } __attribute__((packed));
    struct FarPointer frptr;
    frptr.segment = 0x08;
    frptr.offset = (uint32_t) reload_cs;
    asm (
        "lgdt %0\n"
        "ljmp *%1\n"
        "reload_cs:\n"
        "mov $0x10, %%ax\n"
        "mov %%ax, %%ds\n"
        "mov %%ax, %%es\n"
        "mov %%ax, %%fs\n"
        "mov %%ax, %%gs\n"
        "mov %%ax, %%ss\n"

        :: "m" (gdtr), "m" (frptr)
        : "%ax"
    );
}

void lidt(uint8_t *target, uint16_t size) {
    dtr idtr;
    
    idtr.base = (uint32_t) target;
    idtr.limit = size;

    asm (
        "lidt %0\n"
        "sti"
        :: "m" (idtr)
    );
}