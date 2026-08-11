.global _start

.section .text

; print:
;     addi sp, sp, -16
;     sw ra, 12(sp)
;     sw s0, 8(sp)
;
;     li a0, 0
;     la a1, hello
;     la a2, hello_size
;     li a7, 64
;     ecall
;
;     lw      s0, 8(sp)
;     lw      ra, 12(sp)
;     addi    sp, sp, 16
;     ret

_start:
    li a0, 0
    la a1, hello
    la a2, hello_size
    li a7, 64
    ecall
    li a0, 0
    li a7, 93
    ecall

.section .data
hello: .ascii "Hello World! Welcome to riscv32\n"
    .set hello_size, .-hello

