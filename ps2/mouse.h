#ifndef __PS2_MOUSE_H
#define __PS2_MOUSE_H

#include <stdint.h>
#include "controller.h"
#include "../ports.h"
#include "../pic.h"
#include "../stdutils.h"

typedef struct {
    uint8_t byte1;
} mouse_state;

mouse_state *mouse_init(PS2TOKEN token);
void mouse_in(void);

#endif