#!/usr/bin/bash

filename=${1%.*}
riscv64-suse-linux-as -march=rv32gc -mabi=ilp32 -o $filename.o $1
riscv64-suse-linux-ld -m elf32lriscv_ilp32 -o $filename $filename.o
