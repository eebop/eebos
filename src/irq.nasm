extern isr_handler

%macro isr 1
isr_%+%1: ; we just came in from a int
          ; which pushed the following data to the stack:
          ; EIP CS (zero-padded to dword) EFLAGS
    push esp
    push eax
    push ebx
    push ecx
    push edx
    push ebp
    push esi
    push edi

    lea eax, [esp] ; eax now stores where we placed our data
    push DWORD %1
    
    sub esp, 0x0c ; We need to align with at least 3 slots left
    and esp, 0xFFFFFFEF ; align to nearest sixteenbyte
    add esp, 0x04 ; however, call will push a byte so we need to be off that sixteenbyte
    mov [esp+4], eax ; this is for us to remember where we stored the data
    mov [esp], eax ; this is the *T arg to isr_handler
    call isr_handler
    pop esp ; remember where we placed the data (offset by aligning)

    pop edi
    pop esi
    pop ebp
    pop edx
    pop ecx
    pop ebx
    pop eax
    pop esp
    iretd
%endmacro

%assign i 0
%rep 256
    isr i
    %assign i i+1
%endrep

global isr_table
isr_table:
%assign i 0
%rep 256
    dd isr_%+i
    %assign i i+1
%endrep