#include "gdt.h"

typedef struct __attribute__((packed)) {
    uint32_t base;
    uint16_t limit;
} dtr;


void encodeGDTEntry32(uint8_t *target, GDTSegment source, uint32_t base)
{
    // Check the limit to make sure that it can be encoded
    if (source.limit > 0xFFFFF) {
        // printf("GDT cannot encode limits larger than 0xFFFFF");
        return;
    }
    if (source.is32 && source.is64) {
        return;
    }
    
    // Encode the limit
    target[0] = source.limit & 0xFF;
    target[1] = (source.limit >> 8) & 0xFF;
    target[6] = (source.limit >> 16) & 0x0F;
    
    // Encode the base
    target[2] = base & 0xFF;
    target[3] = (base >> 8) & 0xFF;
    target[4] = (base >> 16) & 0xFF;
    target[7] = (base >> 24) & 0xFF;
    
    // Encode the access byte
    uint8_t access = 0x80;
    access |= source.privilege << 5;
    access |= source.phys << 4;
    access |= source.exec << 3;
    access |= source.direct_conform << 2;
    access |= source.readwrite << 1;
    access |= source.accessed << 0;
    target[5] = access;
    
    // Encode the flags
    uint8_t flags = 0x0;
    flags |= source.is64 << 1;
    flags |= source.is32 << 2;
    flags |= source.granularity << 3;
    target[6] |= flags << 4;
}

void encodeGDTEntry64(uint8_t *target, GDTSegment source, uint64_t base) {
    encodeGDTEntry32(target, source, base & 0xFFFFFFFF);
    uint32_t remainder = base >> 32;
    target[0x8] = (base >> 0x00) & 0xFF;
    target[0x9] = (base >> 0x08) & 0xFF;
    target[0xA] = (base >> 0x10) & 0xFF;
    target[0xB] = (base >> 0x18) & 0xFF;
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

void lgdt(uint8_t *target, uint16_t size) {
    // dtr gdtr;

    // gdtr.limit = size;
    // gdtr.base  = (uint32_t) target;

    uint16_t gdtr[3];
    gdtr[0] = size;
    gdtr[1] = ((uint32_t) target) & 0xFFFF;
    gdtr[2] = ((uint32_t) target) >> 16;
    printf("loading gdt: %x, %x\n", target, size);

    asm (
        "lgdt %0\n"

        :: "m" (gdtr)
    );
}

extern void reload_cs(void);

void reload_segs(uint16_t code, uint16_t data) {
    // struct FarPointer {
    //     uint32_t offset;
    //     uint16_t segment;
    // } __attribute__((packed));
    // struct FarPointer frptr;
    // frptr.segment = code;
    // frptr.offset = (uint32_t) reload_cs;
    // asm (
    //     "jmp %[code], $reload_cs\n"
    //     "reload_cs:\n"
    //     "mov %[data_seg], %%ax\n"
    //     "mov %%ax, %%ds\n"
    //     "mov %%ax, %%es\n"
    //     "mov %%ax, %%fs\n"
    //     "mov %%ax, %%gs\n"
    //     "mov %%ax, %%ss\n"
    //     :: [code] "ir" (code), [data_seg] "g" (data)
    //     : "%ax"

    // );
}

void lidt(uint8_t *target, uint16_t size) {
    dtr idtr;
    
    idtr.base = (uint32_t) target;
    idtr.limit = size;

    asm volatile (
        "lidt %0\n"
        "sti"
        :: "m" (idtr)
    );
}