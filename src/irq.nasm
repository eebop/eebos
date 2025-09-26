extern isr_handler

%macro isr 1
isr_%+%1: ; we just came in from a int
          ; which pushed the following data to the stack:
          ; EIP CS (zero-padded to dword) EFLAGS

    push eax
    
    mov eax, 0
    xchg [stored_sp], eax
    test eax, eax
    jz stack_configured_%+%1
    mov esp, eax ; switch to kernel stack if not already on it

stack_configured_%+%1:
    
    push DWORD [eax - 0x10]
    push DWORD [eax - 0xc ]
    push DWORD [eax - 0x8 ]
    push DWORD [eax - 0x4 ]
    
    sub eax, 0x10 ; eax is now old esp
    push eax

    push ebx
    push ecx
    push edx
    push ebp
    push esi
    push edi

    lea eax, [esp] ; eax now stores where we placed our data

    push DWORD %1 ; no point to pop
    lea ebx, [esp] ; .. and ebx includes the interrupt

    sub esp, 0x0c ; We need to align with at least 3 slots left
    and esp, 0xFFFFFFE0 ; align to nearest thirtytwobyte
    add esp, 0x04 ; however, call will push a dword so we need to be off that thirtytwobyte
    mov [esp+4], eax ; this is for us to remember where we stored the data  
    mov [esp], ebx ; this is the *T arg to isr_handler
    call isr_handler

    xchg bx, bx

    mov esp, [esp+4] ; remember where we placed the data

    pop edi
    pop esi
    pop ebp
    pop edx
    pop ecx
    pop ebx
    pop eax
    iretd
%endmacro

global stored_sp
stored_sp:
; Stores the OS esp to restore
; In the case where an interrupt happened whilst processing an interrupt
; contains a 0
dd 0

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