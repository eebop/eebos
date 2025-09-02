#include "page64.h"
#include "stdutils.h"

void encodePageEntry(uint64_t *buf, pageEntry entry) {
    uint64_t out = (uint64_t) entry.ptr;
    
    if (out & 0xFFF) {
        printf("Page Entry Pointer not aligned to 4KB chunk!\n");
        return;
    }
    if (!entry.is4kb && entry.isLeaf && (out & 0x1FF000)) {
        printf("Page Entry Pointer not aligned to 2MiB chunk!");
        return;
    }
    // No way to check for 1GB aligmnent with minimalistic pageEntry


    out |= 1 << 0; // Present
    out |= entry.writeable << 1;
    out |= entry.userable << 2;
    out |= entry.writeThrough << 3;
    out |= entry.cacheDisable << 4;
    out |= entry.accessed << 5;
    out |= entry.dirty << 6;
    if (entry.is4kb) {
        out |= entry.pat << 7;
    } else {
        out |= entry.isLeaf << 7;
    }
    out |= entry.global << 8;

    if (!entry.is4kb && entry.isLeaf) {
        out |= entry.pat << 12;
    }
    *buf = out;
}


void init_pages() {

    uint64_t *data = malloc_aligned(0x1000, 0x1000);
    printf("got data: %d\n", data);

    pageEntry defaultL4 = {
        .writeable = 1,
        .userable = 1,
        .writeThrough = 1,
        .cacheDisable = 0,
        .accessed = 0,
        .dirty = 0,
        .is4kb = 0,
        .pat = 0,
        .isLeaf = 0,
        .global = 0,
        .disable_exec = 0
    };

    pageEntry defaultPDPT = {
        .writeable = 1,
        .userable = 1,
        .writeThrough = 1,
        .cacheDisable = 1,
        .accessed = 0,
        .dirty = 0,
        .is4kb = 0,
        .pat = 0,
        .isLeaf = 0,
        .global = 0,
        .disable_exec = 0
    };
    
    pageEntry defaultPD = {
        .writeable = 1,
        .userable = 1,
        .writeThrough = 1,
        .cacheDisable = 1,
        .accessed = 0,
        .dirty = 0,
        .is4kb = 0,
        .pat = 0,
        .isLeaf = 1,
        .global = 0,
        .disable_exec = 0
    };

    uint64_t i = 0;// for (uint64_t i=0; i!=12; i++) {
        uint64_t *ptr = malloc_aligned(0x1000, 0x1000);
        defaultL4.ptr = (uint64_t) ptr;
        encodePageEntry(&data[i], defaultL4);
        for (uint64_t j=0; j!=512; j++) {
            // uint64_t physPtr = (j << 30) + (((uint64_t)i) << 39);
            uint64_t *pdptr = malloc_aligned(0x1000, 0x1000);
            defaultPDPT.ptr = (uint64_t) pdptr;
            encodePageEntry(&ptr[j], defaultPDPT);
            for (uint64_t k=0;k!=512;k++) {
                uint64_t physPtr = (i << 39) + (j << 30) + (k << 21);
                defaultPD.ptr = physPtr;
                encodePageEntry(&pdptr[k], defaultPD);
            }
        }
    // }

    uint32_t cpuid;

    asm (
        "mov $0x80000001, %%eax\n"
        "cpuid\n"
        "mov %%edx, %0\n"
        : "=g" (cpuid)
        :: "eax", "ebx", "ecx", "edx"
    );

    uint32_t cr4;
    uint32_t msr;

    // asm (
    //     "mov $0x20, %0\n"
    //     "or $0x20, %0\n"
    //     // "mov %0, %%cr4\n"
    //     // "mov %%cr4, %0\n"

    //     "mov $0xC0000080, %%ecx\n"
    //     "rdmsr\n"
    //     "or $0x100, %%eax\n"
    //     "wrmsr\n"
    //     "rdmsr\n"
    //     "mov %%eax, %1"
    //     : "=r" (cr4), "=r" (msr)
    //     :: "%eax", "%ecx"
    // );
       
    asm (
        "mov %%cr4, %0\n"
        "or $0x20, %0\n"
        "mov %0, %%cr4\n"
        "mov %%cr4, %0"
        : "=r" (cr4)
    );
    printf("%x\n", cr4);
    // printf("cr4 = %x, msr = %x\ncr4(&) %x, msr = %x\n", cr4, msr, cr4 & 0x20, msr & 0x100);

    asm (
        "mov $0xC0000080, %%ecx\n"
        "rdmsr\n"
        "or $0x100, %%eax\n"
        "wrmsr\n"
        "rdmsr\n"
        "mov %%eax, %0"
        : "=r" (msr)
        :: "ecx"
    );

    printf("%x\n", msr);

    uint32_t cr3;

    printf("data is: %x\n", data);

    asm volatile (
        "mov %1, %%cr3\n"
        "mov %%cr3, %0\n"
        : "=g" (cr3)
        : "r" (data)
    );
    asm volatile (
        "mov %%cr0, %%eax\n"
        "or $0x80000000, %%eax\n"
        "mov %%eax, %%cr0\n"
        ::: "%eax"
    );

}

void call64(uint32_t ptr) {
        asm volatile (
        "xchgw %%bx, %%bx\n"
        ".global tramp64\n"
        "jmp $0x18, $tramp64\n"
        "mov $0x55FF55FF, %%eax\n"
        ".code64\n"
        "tramp64:\n"
        "xor %%rbx, %%rbx\n"
        "hlt"
        ::: "%eax"
    );

}