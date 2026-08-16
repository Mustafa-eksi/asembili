#!/usr/bin/bash
set -e

filename=${1%.*}
riscv64-suse-linux-gcc -march=rv32g -mabi=ilp32 -c -Oz $1 -o $filename.o
riscv64-suse-linux-ld -m elf32lriscv_ilp32 -o $filename $filename.o
# riscv32-unknown-elf-gcc \
#     -march=rv32gc \
#     "$1" \
#     -o "$filename"
#     # -mabi=ilp32d \
#     # -static \
#     # -Oz \
#     # -ffreestanding \
#     # -fno-pic \
#     # -nostdlib \
#     # -Wl,--no-relax \
#     # -Wl,-e,_start \
