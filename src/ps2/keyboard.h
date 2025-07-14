#ifndef __PS2_KEYBOARD_H
#define __PS2_KEYBOARD_H

#include <stdint.h>
#include "../ports.h"
#include "../pic.h"
#include "../stdutils.h"

typedef struct {

} keyboard_state;



// keyboard_state *keyboard_init(PS2TOKEN token);
void keyboard_in(void);

#endif