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
    uint32_t cpuid;
    asm volatile (
        "mov $0x80000001, %%eax\n"
        "cpuid\n"
        "mov %%edx, %0\n"
        : "=g" (cpuid)
        :: "eax", "ebx", "ecx", "edx"
    );

    printf("CPUID is %b\n", cpuid);


    uint64_t *data = malloc_aligned(0x1000, 0x1000);

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
        .isLeaf = 1,
        .global = 0,
        .disable_exec = 0
    };
    

    for (uint64_t i=0; i!=512; i++) {
        uint64_t *ptr = malloc_aligned(0x1000, 0x1000);
        defaultL4.ptr = (uint64_t) ptr;
        encodePageEntry(&data[i], defaultL4);
        for (uint64_t j=0; j!=512; j++) {
            uint64_t physPtr = (uint64_t) (i << 39) + (j << 30);
            defaultPDPT.ptr = physPtr;
            encodePageEntry(&ptr[j], defaultPDPT);
        }
    }

    asm volatile (
        "mov %%cr4, %%eax\n"
        "or $0x20, %%eax\n"
        "mov %%eax, %%cr4\n"
        
        "mov $0xC0000080, %%ecx\n"
        "rdmsr\n"
        "or $0x100, %%eax\n"
        "wrmsr\n"
        
        "mov %0, %%cr3\n"

        "mov %%cr0, %%eax\n"
        "or $0x80000000, %%eax\n"
        "mov %%eax, %%cr0\n"
        :: "r" (data)
        : "%eax", "%ecx"
    );

    uint32_t cr = 100;
    asm (
        "mov %%cr0, %0"
        : "=r" (cr)
    );
    printf("cr is: %b\n", cr);

    printf("here!\n");
}