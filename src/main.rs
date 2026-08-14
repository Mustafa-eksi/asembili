#![allow(dead_code)]

use std::io;
use std::io::Write;
use std::path::Path;
use std::env;
use std::fs;
use std::ptr;
use std::cmp::min;
// use std::slice;

use goblin::error;
use goblin::elf::Elf;

use syscalls::riscv32;
use syscalls::syscall;

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

// https://en.wikipedia.org/wiki/RISC-V_instruction_listings
// reg   = 1
// imm   = 1 i32
type RegType = u8;
type ImmType = i32;
#[derive(Debug, Copy, Clone, Default, PartialEq)]
enum Inst {

    // Emulator special instructions
    #[default]
    Exit, // Ends emulation
    Print(RegType),
    Dump,

    // Memory
    LoadWord(RegType, ImmType, RegType),                // reg (out), imm, reg (in addr)
    StoreWord(RegType, ImmType, RegType),               // reg (in), imm, reg (out addr)
    LoadImmediate(RegType, ImmType),                    // reg (out), imm
    LoadUpperImmediate(RegType, ImmType),               // reg (out), imm
    Move(RegType, RegType),                             // reg (out), reg (in)

    // Arithmetic
    AddUpperImmediateToPc(RegType, ImmType),            // reg (out), imm
    AddImmediate(RegType, RegType, ImmType),            // reg (out), reg (in), imm
    Add(RegType, RegType, RegType),                     // reg (out), reg (in), reg (in)
    Subtract(RegType, RegType, RegType),                // reg (out), reg (in), reg (in)
    Multiply(RegType, RegType, RegType),                // reg (out), reg (in), reg (in)
    Divide(RegType, RegType, RegType),                  // reg (out), reg (in), reg (in)
    Remainder(RegType, RegType, RegType),               // reg (out), reg (in), reg (in)

    // Bitwise Operations
    And(RegType, RegType, RegType),                     // reg (out), reg (in), reg (in)
    Or(RegType, RegType, RegType),                      // reg (out), reg (in), reg (in)
    Not(RegType, RegType),                              // reg (out), reg (in)
    Xor(RegType, RegType, RegType),                     // reg (out), reg (in), reg (in)
    XorImm(RegType, RegType, ImmType),                  // reg (out), reg (in), imm
    ShiftLeftLogical(RegType, RegType, RegType),        // reg (out), reg (in), reg (in)
    ShiftRightLogical(RegType, RegType, RegType),       // reg (out), reg (in), reg (in)
    ShiftRightArithmetic(RegType, RegType, RegType),    // reg (out), reg (in), reg (in)

    // Control Flow
    Jump(ImmType),                                      // imm
    JumpAndLink(RegType, ImmType),                      // reg (out), imm
    JumpAndLinkReturn(RegType, RegType, ImmType),       // reg (out), reg (in), imm
    BranchEquals(RegType, RegType, ImmType),            // reg, reg, imm
    BranchNotEquals(RegType, RegType, ImmType),         // reg, reg, imm
    BranchLessThan(RegType, RegType, ImmType),          // reg, reg, imm
    BranchGreaterThan(RegType, RegType, ImmType),       // reg, reg, imm
    BranchLessEq(RegType, RegType, ImmType),            // reg, reg, imm
    BranchGreaterEq(RegType, RegType, ImmType),         // reg, reg, imm
    Ecall,

    // Misc
    Nop,
}

impl TryFrom<u32> for Inst {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value & 0x3 != 3 {
            return Err(());
        }

        let opcode = value & 0x7f;
        let rd = ((value >> 7) & 0x1f) as RegType;
        let funct3 = (value >> 12) & 0x7;
        let rs1 = ((value >> 15) & 0x1f) as RegType;
        let rs2 = ((value >> 20) & 0x1f) as RegType;
        let funct7 = value >> 25;
        let u_imm = ((value as i32) >> 12) << 12;
        let i_imm = (value as i32) >> 20;
        let s_imm = (((value >> 7) & 0x1f) | ((value >> 25) << 5)) as i32;
        let s_imm = (s_imm << 20) >> 20;
        let b_imm = (((value >> 8) & 0x0f) << 1)
            | (((value >> 25) & 0x3f) << 5)
            | (((value >> 7) & 0x01) << 11)
            | (((value >> 31) & 0x01) << 12);
        let b_imm = ((b_imm << 19) as i32) >> 19;
        let j_imm = (((value >> 21) & 0x03ff) << 1)
            | (((value >> 20) & 0x01) << 11)
            | (((value >> 12) & 0x00ff) << 12)
            | (((value >> 31) & 0x01) << 20);
        let j_imm = ((j_imm as i32) << 11) >> 11;

        match (opcode, funct3, funct7) {
            (0x17, _, _) => Ok(Inst::AddUpperImmediateToPc(rd, u_imm)),
            (0x03, 0b010, _) => Ok(Inst::LoadWord(rd, i_imm, rs1)),
            (0x23, 0b010, _) => Ok(Inst::StoreWord(rs2, s_imm, rs1)),
            (0x37, _, _) => Ok(Inst::LoadUpperImmediate(rd, u_imm)),
            (0x13, 0b000, _) => Ok(Inst::AddImmediate(rd, rs1, i_imm)),
            (0x13, 0b100, _) => Ok(Inst::XorImm(rd, rs1, i_imm)),
            (0x33, 0b000, 0b0000000) => Ok(Inst::Add(rd, rs1, rs2)),
            (0x33, 0b100, 0b0000000) => Ok(Inst::Xor(rd, rs1, rs2)),
            (0x33, 0b110, 0b0000000) => Ok(Inst::Or(rd, rs1, rs2)),
            (0x33, 0b111, 0b0000000) => Ok(Inst::And(rd, rs1, rs2)),
            (0x63, 0b100, _) => Ok(Inst::BranchLessThan(rs1, rs2, b_imm)),
            (0x63, 0b001, _) => Ok(Inst::BranchNotEquals(rs1, rs2, b_imm)),
            (0x73, _, _) => Ok(Inst::Ecall),
            (0x67, 0x0, _) => Ok(Inst::JumpAndLinkReturn(rd, rs1, i_imm)),
            (0x6f, _, _) => Ok(Inst::JumpAndLink(rd, j_imm)),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct VirtualMemory {
    raw_pointer: *mut u8,
    size: usize,
    offset: usize,
    flags: u32,
}

type RegisterType = u32;
#[derive(Debug)]
struct Cpu {
    program_memory: [Inst; PROGRAM_SIZE],
    tick: RegisterType, // tick counter
    pc: usize, // program counter
    program_start: *mut u8, // program address
    entry_address: *mut u8,
    x: [RegisterType; REGISTER_COUNT], // general use registers
                    // TODO: introduce saved and temporary registers.
    heap: Vec<RegisterType>,
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
    fn run_inst(&mut self) -> bool {
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
                }.unwrap();

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
                todo!("{:?}", self.program_memory[self.pc]);
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

    fn run(&mut self) {
        while self.program_memory[self.pc] != Inst::Exit {
            // println!("Stepping, pc = {}, inst = {:?}, regs: {:?}", self.pc,
            //     self.program_memory[self.pc], self.x);
            self.step();
        }
    }

    fn debug_mode(&mut self) {
        while self.program_memory[self.pc] != Inst::Exit {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            if line.trim() == "reg" {
                println!("Regs: {:?}", self.x);
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
            } else if line.trim() == "dump" {
                self.dump();
            } else if line.trim() == "exit" {
                return;
            }
        }
        self.run();
    }

    fn set_instructions(&mut self, program: Vec<Inst>) {
        for (pc, inst) in program.into_iter().enumerate() {
            self.program_memory[pc] = inst;
        }
    }

    fn load_elf(&mut self, buffer: &Vec<u8>) {
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
            // println!("{:?}", header);
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
            .map(|inst| Inst::try_from(inst).expect(format!("inst {inst:08x?}").as_str()))
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

fn main() -> error::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Insufficent arguments");
    }
    let path = Path::new(args[1].as_str());
    let buffer: Vec<u8> = fs::read(path)?;
    let mut cpu = Cpu::default();
    cpu.load_elf(&buffer);
    // println!("{:?}", &cpu.program_memory[0..32]);
    if args.into_iter().find(|x| x == "-d").is_some() {
        cpu.debug_mode();
    } else {
        cpu.run();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_implemented_rv32_instructions() {
        assert_eq!(Inst::try_from(0x008000ef), Ok(Inst::JumpAndLink(1, 8)));
        assert_eq!(Inst::try_from(0xffdff0ef), Ok(Inst::JumpAndLink(1, -4)));
        assert_eq!(
            Inst::try_from(0xffc100e7),
            Ok(Inst::JumpAndLinkReturn(1, 2, -4))
        );
        assert_eq!(
            Inst::try_from(0x12345097),
            Ok(Inst::AddUpperImmediateToPc(1, 0x12345000))
        );
        assert_eq!(
            Inst::try_from(0xfffff097),
            Ok(Inst::AddUpperImmediateToPc(1, -0x1000))
        );
        assert_eq!(
            Inst::try_from(0xfff10093),
            Ok(Inst::AddImmediate(1, 2, -1))
        );
        assert_eq!(Inst::try_from(0xff010113), Ok(Inst::AddImmediate(2, 2, -16)));
        assert_eq!(Inst::try_from(0x003100b3), Ok(Inst::Add(1, 2, 3)));
        assert_eq!(Inst::try_from(0x00000073), Ok(Inst::Ecall));
        assert_eq!(Inst::try_from(0x003140b3), Ok(Inst::Xor(1, 2, 3)));
        assert_eq!(Inst::try_from(0xfe0696e3), Ok(Inst::BranchNotEquals(13, 0, -20)));
        assert_eq!(Inst::try_from(0x003160b3), Ok(Inst::Or(1, 2, 3)));
        assert_eq!(Inst::try_from(0x003170b3), Ok(Inst::And(1, 2, 3)));
        assert_eq!(Inst::try_from(0xfff14093), Ok(Inst::XorImm(1, 2, -1)));
        assert_eq!(Inst::try_from(0xffc12083), Ok(Inst::LoadWord(1, -4, 2)));
        assert_eq!(Inst::try_from(0xfe312e23), Ok(Inst::StoreWord(3, -4, 2)));
        assert_eq!(
            Inst::try_from(0xfe314ce3),
            Ok(Inst::BranchLessThan(2, 3, -8))
        );
    }

    #[test]
    fn rejects_unimplemented_or_invalid_instructions() {
        assert_eq!(Inst::try_from(0x40310033), Err(())); // SUB
        assert_eq!(Inst::try_from(0x00000000), Err(())); // Not a 32-bit instruction
    }

    #[test]
    fn test_add_upper_immediate_to_pc() {
        let mut cpu = Cpu::default();
        cpu.pc = 2;
        cpu.program_start = 0x10000 as *mut u8;
        let pc_addr = cpu.program_start as usize+cpu.pc*4;
        cpu.program_memory[2] = Inst::AddUpperImmediateToPc(1, 0x12345000);
        cpu.run_inst();
        assert_eq!(cpu.x[1], pc_addr as u32+0x12345000);

        cpu.program_memory[2] = Inst::AddUpperImmediateToPc(1, -0x1000);
        cpu.run_inst();
        assert_eq!(cpu.x[1], pc_addr as u32-0x1000);
    }

    #[test]
    fn test_fibonacci() {
        let mut cpu: Cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 10, 0),       // a
            Inst::AddImmediate(1, 1, 1),       // b
            Inst::AddImmediate(4, 4, 15),      // limit
            Inst::Add(12, 10, 1),             // c = a + b
            Inst::Move(10, 1),               // a = b
            Inst::Move(1, 12),               // b = c
            Inst::AddImmediate(3, 3, 1),       // i += 1
            Inst::BranchLessThan(3, 4, -5*4), // go up 4 if c < b
        ]);
        cpu.run();
        assert_eq!(cpu.x[1], 987);
    }

    #[test]
    fn test_memory() {
        let mut cpu: Cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 10, 69), // val
            Inst::AddImmediate(1, 1, 100), // addr
            Inst::StoreWord(10, 0, 1),
            Inst::LoadWord(12, 0, 1),
        ]);
        cpu.run();
        assert_eq!(cpu.x[12], 69);
    }

    #[test]
    fn test_rule110() {
        let mut cpu: Cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(12, 0, 1),       // First 1
            Inst::AddImmediate(3, 0, 0),       // Addr
            Inst::StoreWord(12, 0, 3),       // store
            Inst::AddImmediate(15, 0, 32),      // limit
            Inst::AddImmediate(1, 0, 1),       // start i from 1

            // Outer Loop Start
            Inst::Xor(4, 4, 4),             // set j = 0
            Inst::Xor(5, 5, 5),             // set left = 0
            Inst::Xor(6, 6, 6),             // set center = 0
            Inst::Xor(7, 7, 7),             // set right = 0
            Inst::LoadWord(6, 0, 4),

                // Inner Loop start
                Inst::Move(8, 4),
                Inst::AddImmediate(8, 8, 1),
                Inst::LoadWord(7, 0, 8),

                Inst::Xor(9, 6, 5),
                Inst::XorImm(10, 7, 1),
                Inst::And(10, 5, 10),
                Inst::Or(11, 9, 10),
                Inst::StoreWord(11, 0, 4),

                Inst::Move(5, 6),
                Inst::Move(6, 7),
                Inst::AddImmediate(4, 4, 1),       // j += 1
                Inst::BranchLessEq(4, 1, -12*4), // go up if j < i (size)

            Inst::AddImmediate(1, 1, 1),       // i += 1
            Inst::BranchLessThan(1, 15, -19*4), // go up if i < limit
        ]);
        cpu.run();

        assert_eq!(&cpu.heap[0..32], [1, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1]);
    }

    #[test]
    fn test_and() {
        let mut cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 10, 1),
            Inst::AddImmediate(1, 1, 0),
            Inst::And(3, 1, 1),
            Inst::And(4, 1, 10),
            Inst::And(5, 10, 1),
            Inst::And(6, 10, 10),
            Inst::AddImmediate(7, 7, 5),
            Inst::AddImmediate(8, 8, 2),
            Inst::And(9, 7, 8),
        ]);
        cpu.run();
        assert_eq!(cpu.x[3], 0);
        assert_eq!(cpu.x[4], 0);
        assert_eq!(cpu.x[5], 0);
        assert_eq!(cpu.x[6], 1);
        assert_eq!(cpu.x[9], 0);
    }

    #[test]
    fn test_or() {
        let mut cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 0, 1),
            Inst::AddImmediate(1, 1, 0),
            Inst::Or(3, 1, 1),
            Inst::Or(4, 1, 10),
            Inst::Or(5, 10, 1),
            Inst::Or(6, 10, 10),
            Inst::AddImmediate(7, 7, 5),
            Inst::AddImmediate(8, 8, 2),
            Inst::Or(9, 7, 8),
        ]);
        cpu.run();
        assert_eq!(cpu.x[3], 0);
        assert_eq!(cpu.x[4], 1);
        assert_eq!(cpu.x[5], 1);
        assert_eq!(cpu.x[6], 1);
        assert_eq!(cpu.x[9], 7);
    }

    #[test]
    fn test_xor() {
        let mut cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 10, 1),
            Inst::AddImmediate(1, 1, 0),
            Inst::Xor(3, 1, 1),
            Inst::Xor(4, 1, 10),
            Inst::Xor(5, 10, 1),
            Inst::Xor(6, 10, 10),
            Inst::AddImmediate(7, 7, 5),
            Inst::AddImmediate(8, 8, 3),
            Inst::Xor(9, 7, 8),
        ]);
        cpu.run();
        assert_eq!(cpu.x[3], 0);
        assert_eq!(cpu.x[4], 1);
        assert_eq!(cpu.x[5], 1);
        assert_eq!(cpu.x[6], 0);
        assert_eq!(cpu.x[9], 6);
    }

    #[test]
    fn test_xor2() {
        let mut cpu = Cpu::default();
        cpu.set_instructions(vec![
            Inst::AddImmediate(10, 10, 1),
            Inst::AddImmediate(1, 1, 0),
            Inst::Xor(3, 1, 10),
            Inst::Xor(4, 10, 10),
        ]);
        cpu.run();
        assert_eq!(cpu.x[3], 1);
        assert_eq!(cpu.x[4], 0);
    }
}
