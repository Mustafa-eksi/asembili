use std::io;
use std::io::Write;
use std::ptr;
use std::cmp::min;

use goblin::elf::Elf;

use syscalls::riscv32;
use syscalls::syscall;

use crate::inst::*;

use crate::virtual_memory::VirtualMemory;

// 64 KB is enough for everyone
const HEAP_SIZE: usize = 64e3 as usize;
const PROGRAM_SIZE: usize = 1024;
const REGISTER_COUNT: usize = 32;

#[repr(usize)]
enum Registers {
    Zero            = 0,
    ReturnAddress   = 1,
    StackPointer    = 2,
    GlobalPointer   = 3,
    ThreadPointer   = 4,
    Temporary0      = 5,
    Temporary1      = 6,
    Temporary2      = 7,
    FramePointer    = 8,
    Saved1          = 9,
    Argument0       = 10,
    Argument1       = 11,
    Argument2       = 12,
    Argument3       = 13,
    Argument4       = 14,
    Argument5       = 15,
    Argument6       = 16,
    Argument7       = 17,
    Saved2          = 18,
    Saved3          = 19,
    Saved4          = 20,
    Saved5          = 21,
    Saved6          = 22,
    Saved7          = 23,
    Saved8          = 24,
    Saved9          = 25,
    Saved10         = 26,
    Saved11         = 27,
    Temporary3      = 28,
    Temporary4      = 29,
    Temporary5      = 30,
    Temporary6      = 31,
}

type RegisterType = u32;
#[derive(Debug)]
pub struct Cpu {
    pub program_memory: [Inst; PROGRAM_SIZE],
    tick: RegisterType, // tick counter
    pub pc: usize, // program counter
    pub program_start: *mut u8, // program address
    entry_address: *mut u8,
    pub x: [RegisterType; REGISTER_COUNT], // general use registers
                    // TODO: introduce saved and temporary registers.
    pub heap: Vec<RegisterType>,
    virtmems: Vec<VirtualMemory>,
}

impl Default for Cpu {
    fn default() -> Self {
        let mut c = Cpu {
            program_memory: [Inst::Exit; PROGRAM_SIZE],
            tick: 0, pc: 0, program_start: 0 as *mut u8,
            entry_address: 0 as *mut u8,
            x: [0; REGISTER_COUNT],
            heap: vec![0; HEAP_SIZE],
            virtmems: Vec::new()
        };
        c.x[2] = HEAP_SIZE as RegisterType;
        c
    }
}

impl Cpu {
    // true = increase pc
    // false = don't change pc (means we made a jump)
    pub fn run_inst(&mut self) -> bool {
        match self.program_memory[self.pc] {
            Inst::AddUpperImmediateToPc(reg, imm) => {
                let pc_addr = self.program_start as usize+self.pc*4;
                self.x[reg as usize] = (pc_addr as RegisterType).wrapping_add(imm as RegisterType);
            },
            Inst::AddImmediate(reg1, reg2, imm) => {
                self.x[reg1 as usize] = (self.x[reg2 as usize] as i64 + imm as i64) as RegisterType;
            },
            Inst::Add(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] + self.x[reg3 as usize];
            },
            Inst::Move(reg1, reg2) => {
                self.x[reg1 as usize] = self.x[reg2 as usize];
            },
            Inst::BranchLessThan(reg1, reg2, imm) => {
                if self.x[reg1 as usize] < self.x[reg2 as usize] {
                    self.pc = (self.pc as isize + (imm/4) as isize) as usize;
                }
            },
            Inst::BranchNotEquals(reg1, reg2, imm) => {
                if self.x[reg1 as usize] != self.x[reg2 as usize] {
                    self.pc = (self.pc as isize + (imm/4) as isize - 1) as usize;
                }
            },
            Inst::Print(reg) => {
                println!("{}", self.x[reg as usize]);
            },
            Inst::StoreWord(input, offset, addr_reg) => {
                // TODO: expand array when needed
                let addr = self.x[addr_reg as usize];
                self.heap[(addr as i64 + offset as i64) as usize] = self.x[input as usize];
            },
            Inst::LoadWord(reg, offset, addr_reg) => {
                let addr = self.x[addr_reg as usize];
                // println!("Load from {}, load to {}, offset {}, loaded value {}", addr, reg, offset, self.heap[(addr as i64 + offset as i64) as usize]);
                self.x[reg as usize] = self.heap[(addr as i64 + offset as i64) as usize];
            },
            Inst::LoadUpperImmediate(reg, imm) => {
                self.x[reg as usize] = (imm<<12) as u32;
            },
            Inst::Xor(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] ^ self.x[reg3 as usize];
            },
            Inst::XorImm(reg1, reg2, imm) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] ^ imm as RegisterType;
            },
            Inst::BranchLessEq(reg1, reg2, imm) => {
                if self.x[reg1 as usize] <= self.x[reg2 as usize] {
                    self.pc = (self.pc as isize + (imm/4) as isize) as usize;
                }
            },
            Inst::Not(reg1, reg2) => {
                self.x[reg1 as usize] = (!(self.x[reg2 as usize] as RegisterType)) as RegisterType;
            },
            Inst::And(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] & self.x[reg3 as usize];
            },
            Inst::Or(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] | self.x[reg3 as usize];
            },
            Inst::Ecall => {
                let syscall_no = riscv32::Sysno::try_from(self.x[Registers::Argument7 as usize]).unwrap();
                let x86_no = syscall_no.name().parse().unwrap();
                // self.dump();
                let _output = unsafe {
                    syscall!(
                        x86_no,
                        self.x[Registers::Argument0 as usize],
                        self.x[Registers::Argument1 as usize],
                        self.x[Registers::Argument2 as usize],
                        self.x[Registers::Argument3 as usize],
                        self.x[Registers::Argument4 as usize],
                        self.x[Registers::Argument5 as usize]
                        )
                }.map_err(|err| {
                    // self.dump
                    panic!("{:?}", err);
                });

                // println!("{:?}", syscall_no);
            },
            Inst::JumpAndLinkReturn(rd, rs1, imm) => {
                // TODO: This might be disastrous
                self.x[rd as usize] = (self.pc + 1) as u32;
                self.pc = (self.x[rs1 as usize] as i64 + (imm as i64)/4) as usize;
                return false;
                // self.dump();
            },
            Inst::JumpAndLink(rd, imm) => {
                self.x[rd as usize] = (self.pc + 1) as u32;
                self.pc = (self.pc as i64 + (imm as i64)/4) as usize;
                return false;
            },
            Inst::Dump => {
                self.dump();
            },
            _ => {
                todo!("Couldn't run instruction {:?}", self.program_memory[self.pc]);
            }
        };
        true
    }

    fn step(&mut self) {
        self.tick += 1;
        self.x[0] = 0; // FIXME: This is just a hack needs further inspection
        if self.run_inst() {
            self.pc += 1;
        }
    }

    pub fn run(&mut self) {
        while self.program_memory[self.pc] != Inst::Exit {
            // println!("Stepping, pc = {}, inst = {:?}, regs: {:?}", self.pc,
            //     self.program_memory[self.pc], self.x);
            self.step();
        }
    }

    pub fn debug_mode(&mut self) {
        while self.program_memory[self.pc] != Inst::Exit {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            if line.trim() == "reg" {
                println!("Regs: {:x?}", self.x);
            } else if line.trim() == "n" {
                println!("{}: {:?}", self.pc, self.program_memory[self.pc]);
                self.step();
            } else if line.trim() == "heap" {
                println!("Heap: {:?}", &self.heap[0..32]);
            } else if line.trim() == "inst" {
                println!("{}: {:?}", self.pc, self.program_memory[self.pc]);
            } else if line.trim() == "insts" {
                for (i, inst) in self.program_memory.into_iter().enumerate() {
                    if inst == Inst::Exit {
                        break;
                    }
                    println!("{}: {:?}", i, inst);
                }
            } else if line.trim() == "pc" {
                let pc_addr = self.program_start as usize+self.pc*4;
                println!("pc: {}, program_address: {pc_addr:08x?}", self.pc);
            } else if line.trim() == "clear" {
                print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
            } else if line.trim() == "run" {
                break;
            } else if line.trim() == "virtmem" {
                for virtmem in self.virtmems.clone() {
                    println!("{virtmem:?}");
                    for x in (virtmem.raw_pointer as usize)..(virtmem.raw_pointer as usize+virtmem.size) {
                        unsafe {
                            print!("{:02x?} ", *(x as *mut u8));
                        }
                        if x % 16 == 0 {
                            println!();
                            print!("{x:x?}\t");
                        }
                    }
                    println!();
                }
            } else if line.trim() == "dump" {
                self.dump();
            } else if line.trim() == "exit" {
                return;
            }
        }
        self.run();
    }

    pub fn set_instructions(&mut self, program: Vec<Inst>) {
        for (pc, inst) in program.into_iter().enumerate() {
            self.program_memory[pc] = inst;
        }
    }

    pub fn load_elf(&mut self, buffer: &Vec<u8>) {
        let elf = Elf::parse(&buffer).unwrap();
        let mut text_off: usize = 0;
        let mut text_size: usize = 0;
        for section in elf.section_headers {
            let name = elf.shdr_strtab.get_at(section.sh_name).unwrap();
            if name == ".text" {
                text_off = section.sh_offset as usize;
                text_size = section.sh_size as usize;
            }
        }
        for header in elf.program_headers {
            if header.p_type != 1 || header.p_memsz == 0 {
                continue;
            }
            let mut virtmem = VirtualMemory::default();
            virtmem.raw_pointer = header.p_vaddr as *mut u8;
            virtmem.size = header.p_memsz as usize;
            virtmem.flags = header.p_flags;
            virtmem.offset = header.p_offset as usize;
            self.virtmems.push(virtmem);
        }

        for vmem in self.virtmems.clone() {
            let aligned = ((vmem.raw_pointer as u32)/4096)*4096;
            let aligned_diff = vmem.raw_pointer as u32-aligned;
            let output = unsafe {
                 syscall!(syscalls::Sysno::mmap,
                    aligned, vmem.size as u32+aligned_diff, 0x7,
                    0x20 | 0x02 | 0x100000,
                    usize::MAX,
                    0usize
                    )
            };
            match output {
                Ok(pointer) => {
                    if pointer != vmem.raw_pointer as usize {
                        // println!("Virtual memory allocation aligned {pointer:08x?}");
                    }
                },
                Err(err) => {
                    eprintln!("virtmem pointer: {:?}, virtmem size: {:?}",
                        vmem.raw_pointer, vmem.size);
                    panic!("Virtual memory allocation error {err}");
                },
            }
            unsafe {
                ptr::copy_nonoverlapping(buffer.as_ptr().add(vmem.offset), vmem.raw_pointer,
                min(vmem.size, buffer.len()));
            }
        }

        self.program_start = (text_off+0x10000) as *mut u8;
        self.entry_address = elf.entry as *mut u8;
        self.pc = ((self.entry_address as usize)-self.program_start as usize)/4;
        self.set_instructions(buffer[text_off..text_off+text_size]
            .to_vec()
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .map(|inst| Inst::try_from(inst).expect(format!("inst 0x{inst:08x?}").as_str()))
            .collect());
    }

    fn dump(&self) {
        println!("--- Emulation Dump");
        println!("Stepping, pc = {}, inst = {:?}, tick = {}", self.pc,
            self.program_memory[self.pc], self.tick);
        println!("Regs: {:?}", self.x);
        println!("Instructions: {:?}", &self.program_memory[0..32]);
        println!("Heap: {:?}", &self.heap[0..32]);
        println!("---");
    }
}
