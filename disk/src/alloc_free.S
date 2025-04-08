; allocate and free 1024 bytes of memory
[bits 64]

global _start
section .text
_start:
    mov rax, 18               ; syscall number for ALLOCATE
    mov rdi, 1024             ; size of memory to allocate
    mov rsi, 1                ; align
    int 0x80

    mov rdi, rax              ; address of allocated memory
    mov rax, 19               ; syscall number for FREE
    mov rsi, 1024             ; size of memory to free
    mov rdx, 1                ; align
    int 0x80

    mov rax, 6                ; syscall number for EXIT
    mov rdi, 0                ; no error
    int 0x80
