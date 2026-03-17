global _start

_start:
mov eax, 'abcd'
mov [0xB8000], eax
out 0xe9, al
jmp _start
