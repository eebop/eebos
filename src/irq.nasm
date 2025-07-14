extern badexcept

%macro isr_err_stub 1
isr_stub_%+%1:
    call badexcept
    iret
%endmacro

%macro isr_err 2
isr_stub_%+%1:
    extern %2
    pusha
    call %2
    popa
    iret
%endmacro

isr_err_stub 0
isr_err_stub 1
isr_err_stub 2
isr_err_stub 3
isr_err_stub 4
isr_err_stub 5
isr_err_stub 6
isr_err_stub 7
isr_err_stub 8
isr_err_stub 9
isr_err_stub 10
isr_err_stub 11
isr_err_stub 12
isr_err_stub 13
isr_err_stub 14
isr_err_stub 15
isr_err_stub 16
isr_err_stub 17
isr_err_stub 18
isr_err_stub 19
isr_err_stub 20
isr_err_stub 21
isr_err_stub 22
isr_err_stub 23
isr_err_stub 24
isr_err_stub 25
isr_err_stub 26
isr_err_stub 27
isr_err_stub 28
isr_err_stub 29
isr_err_stub 30
isr_err_stub 31
isr_err_stub 32
isr_err_stub 33; isr_err 33, keyboard_in
isr_err_stub 34
isr_err_stub 35
isr_err_stub 36
isr_err_stub 37
isr_err_stub 38
isr_err_stub 39
isr_err_stub 40
isr_err_stub 41
isr_err_stub 42
isr_err_stub 43
isr_err_stub 44;isr_err 44, mouse_in

global isr_table
isr_table:
%assign i 0 
%rep 45
    dd isr_stub_%+i
    %assign i i+1 
%endrep