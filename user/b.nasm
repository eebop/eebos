global _start

_start:
mov eax, 'dcba'
mov [0xB8000], eax
out 0xe9, al
int3
jmp _start
