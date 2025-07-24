#ifndef __PAGE64_H
#define __PAGE64_H
#include <stdint.h>
#include <stdbool.h>

typedef struct {
    bool present : 1;
    bool writeable : 1;
    bool userable : 1;
    bool writeThrough : 1;
    bool cacheDisable : 1;
    bool accessed : 1;
    bool identity : 1;
    bool dirty : 1;
    bool isLeaf : 1;
    bool is4kb : 1;
    bool pat : 1;
    bool global : 1;
    bool disable_exec : 1;
    uint64_t ptr; // must be aligned
} pageEntry;

void encodePageEntry(uint64_t *buf, pageEntry entry);

void init_pages();

#endif