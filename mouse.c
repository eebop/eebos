#include "mouse.h"
#include "ports.h"
#include "pic.h"
#include "stdutils.h"

void await_safe_write(void) {
    uint8_t value;
    do {
        value = inb(0x64);
        // printf("value: %b\n", value);
    } while (value & 0x2);
}

uint8_t safe_read(void) {
    uint8_t value;
    do {
        value = inb(0x64);
        //printf("safe_read value: %b\n", value);
    } while (! (value & 0x1));
    return inb(0x60);
}

void _send_mouse_byte(uint8_t byte) {
    await_safe_write();
    outb(0x64, 0xD4);
    await_safe_write();
    outb(0x60, byte);

}

void _send_keyboard_byte(uint8_t byte) {
    await_safe_write();
    outb(0x60, byte);

}

void _send_controller_byte(uint8_t byte) {
    await_safe_write();
    outb(0x64, byte);

}

void send_await(uint8_t byte) {
    uint8_t responce;
    _send_mouse_byte(byte);
    do {
        responce = safe_read();
        printf("response: 0x%x\n", responce);
    } while (responce != 0xFA);

}

void enable_streaming(void) {
    // send_await(0xF5);
    // send_await(0xF2);
    // for (int i=0;i!=100;i++) {io_wait();}
    // printf("got: %x\n", safe_read());

    // send_await(0xF4);

    // _send_mouse_byte(0xA8);

    // for (int i=0;i!=100;i++) {io_wait();}
    // printf("self_test: %x\n", safe_read());


    // _send_mouse_byte(0xF4);
    // do {
    //     responce = safe_read();
    //     printf("response: 0x%x\n", responce);
    // } while (responce != 0xFA);

    // _send_mouse_byte(0xF2);
    // do {
    //     responce = safe_read();
    //     printf("responce: 0x%x\n", responce);
    // } while (responce != 0xFA);
    // printf("mouseID: %x\n", safe_read());
   
    // _send_mouse_byte(0xEA);
    // do {
    //     responce = safe_read();
    //     printf("responce: 0x%x\n", responce);
    // } while (responce != 0xFA);
    // printf("mouseID: %x\n", safe_read());

    // _send_mouse_byte(0x01);
    // do {
    //     responce = safe_read();
    //     printf("responce: 0x%x\n", responce);
    // } while (responce != 0xFA);

}

void get_mouse(void) {
    // uint8_t responce; 
    // // printf("here\n");
    // _send_mouse_byte(0xEB);
    // do {
    //     responce = safe_read();
    //     // printf("responce: %x\n", responce);
    // } while (responce != 0xFA);
    // // printf("here\n");
    // uint8_t out1 = safe_read();
    // uint8_t out2 = safe_read();
    // uint8_t out3 = safe_read();
    // uint8_t out4 = safe_read();
    // printf("got: %b (0x%x) %x %x\n", out1, out1, out2, out3);
    // for (int x = 0; x!= 10000000; x++) {
    //     io_wait();
    // }
    // while (1) {}
}

// void reset_mouse(void) {
//     _send_mouse_byte(0xFF);
//     uint8_t byte;
//     do {
//         byte = safe_read();
//     } while(byte != )
// }

void pc2_init(void) {
    for (int x =0; x!= 10000000;x++) {io_wait();}
    _send_controller_byte(0xAD);
    _send_controller_byte(0xA7);

    _send_controller_byte(0xAA);
    uint8_t self_test = safe_read();
    printf("Self Test Responce (should be 0x55): 0x%x\n", self_test);

    inb(0x60);
    _send_controller_byte(0x20);
    uint8_t ccb = safe_read();
    printf("CCB = %b\n", ccb);
    ccb &= ~((1 << 0) + (1 << 4) + (1 << 6));
    ccb |= 3;

    _send_controller_byte(0x60);
    _send_keyboard_byte(ccb);
    _send_controller_byte(0x20);
    uint8_t excess = safe_read();
    // ccb = safe_read();
    printf("ccb: %x, ps/2 2: (0=enabled) %b\n", ccb, ccb & (1 << 5));
    printf("excess: %x\n", excess);
    // _send_controller_byte(0xA8);
    // _send_controller_byte(0xA7);
    _send_controller_byte(0xAB);
    uint8_t test_result = safe_read();
    printf("test result for port 1 (should be 0): %b\n", test_result);
    safe_read();
    // _send_controller_byte(0xA9);
    // test_result = safe_read();
    // printf("test result for port 2 (should be 0): %b\n", test_result);

    // _send_keyboard_byte(0xFF);
    // printf("keyboard responce: %x %x\n", safe_read(), safe_read());

    // _send_mouse_byte(0xFF);
    // printf("mouse responce: %x %x\n", safe_read(), safe_read());


    // _send_mouse_byte(0xF6);

    // // _send_keyboard_byte(0xF4);
    // _send_mouse_byte(0xF4);

    // _send_controller_byte(0xAE);
    // _send_controller_byte(0xA8);


    IRQ_clear_mask(1);
    
    IRQ_clear_mask(12);

    while (1) {};

}