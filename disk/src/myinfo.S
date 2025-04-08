; display the heap size of the program and exit
[bits 64]

section .data
fname: db "/proc/1/heap_size"
len: equ $-fname
buffer: times 256 db 0

global _start
section .text
_start:
    mov rax, 3                ; syscall number for OPEN
    mov rdi, fname            ; addr of string
    mov rsi, len              ; size of string
    mov rdx, 0                ; flags
    int 0x80

    mov rdi, rax              ; file descriptor
    mov rax, 1                ; syscall number for READ
    mov rsi, buffer           ; addr of buffer
    mov rdx, 256              ; size of buffer
    int 0x80

    mov rax, 2
    mov rdi, 1                ; standard output
    mov rsi, buffer           ; addr of string
    mov rdx, len              ; size of string
    int 0x80

    mov rax, 6                ; syscall number for EXIT
    mov rdi, 0                ; no error
    int 0x80
