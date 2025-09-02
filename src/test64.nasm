[BITS 64]

global main64

main64:

xor rax, rax

mov DWORD [0xB8000], 0xFFFFFFFF

hlt