#!/usr/bin/bash
set -e

filename=${1%.*}
# ~/.opt/riscv/bin/riscv32-unknown-elf-gcc -static -Oz $1 -o $filename
riscv64-suse-linux-gcc -march=rv32g -mabi=ilp32 -c -Oz $1 -o $filename.o
riscv64-suse-linux-ld -m elf32lriscv_ilp32 -o $filename $filename.o
