#![allow(dead_code)]

use std::io;
use std::io::Write;

// 64 KB is enough for everyone
const HEAP_SIZE: usize = 64e3 as usize;
const PROGRAM_SIZE: usize = 1024;
const REGISTER_COUNT: usize = 16;

// https://en.wikipedia.org/wiki/RISC-V_instruction_listings
// reg   = 1
// imm   = 1 i8
type RegType = u8;
type ImmType = i8;
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
    Move(RegType, RegType),                             // reg (out), reg (in)

    // Arithmetic
    AddImmediate(RegType, ImmType),                     // reg (out), imm
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
    BranchEquals(RegType, RegType, ImmType),            // reg, reg, imm
    BranchLessThan(RegType, RegType, ImmType),          // reg, reg, imm
    BranchGreaterThan(RegType, RegType, ImmType),       // reg, reg, imm
    BranchLessEq(RegType, RegType, ImmType),            // reg, reg, imm
    BranchGreaterEq(RegType, RegType, ImmType),         // reg, reg, imm

    // Misc
    Nop,
}

#[derive(Debug)]
struct CPU {
    program_memory: [Inst; PROGRAM_SIZE],
    tick: u32, // tick counter
    pc: usize, // program counter
    ra: u32, // return address
    sp: u32, // stack pointer
    gp: u32, // global pointer
    // Skipping tp (thread pointer)
    fp: u32, // frame pointer
    x: [i32; REGISTER_COUNT], // general use registers
                    // TODO: introduce saved and temporary registers.
    heap: Vec<i32>,
}

impl Default for CPU {
    fn default() -> Self {
        CPU {
            program_memory: [Inst::Exit; PROGRAM_SIZE],
            tick: 0, pc: 0, ra: 0, sp: 0, gp: 0, fp: 0,
            x: [0; REGISTER_COUNT],
            heap: vec![0; HEAP_SIZE],
        }
    }
}

impl CPU {
    fn run_inst(&mut self) {
        match self.program_memory[self.pc] {
            Inst::AddImmediate(reg, imm) => {
                self.x[reg as usize] += imm as i32;
            },
            Inst::Add(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] + self.x[reg3 as usize];
            },
            Inst::Move(reg1, reg2) => {
                self.x[reg1 as usize] = self.x[reg2 as usize];
            },
            Inst::BranchLessThan(reg1, reg2, imm) => {
                if self.x[reg1 as usize] < self.x[reg2 as usize] {
                    self.pc = (self.pc as isize + imm as isize) as usize;
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
            Inst::Xor(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] ^ self.x[reg3 as usize];
            },
            Inst::XorImm(reg1, reg2, imm) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] ^ imm as i32;
            },
            Inst::BranchLessEq(reg1, reg2, imm) => {
                if self.x[reg1 as usize] <= self.x[reg2 as usize] {
                    self.pc = (self.pc as isize + imm as isize) as usize;
                }
            },
            Inst::Not(reg1, reg2) => {
                self.x[reg1 as usize] = (!(self.x[reg2 as usize] as u64)) as i32;
            },
            Inst::And(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] & self.x[reg3 as usize];
            },
            Inst::Or(reg1, reg2, reg3) => {
                self.x[reg1 as usize] = self.x[reg2 as usize] | self.x[reg3 as usize];
            },
            Inst::Dump => {
                self.dump();
            },
            _ => {
                todo!("{:?}", self.program_memory[self.pc]);
            }
        }
    }

    fn step(&mut self) {
        self.tick += 1;
        self.run_inst();
        self.pc += 1;
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
                let mut i = 0;
                for inst in self.program_memory {
                    if inst == Inst::Exit {
                        break;
                    }
                    println!("{}: {:?}", i, inst);
                    i += 1;
                }
            } else if line.trim() == "clear" {
                print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
            } else if line.trim() == "dump" {
                self.dump();
            } else if line.trim() == "exit" {
                return;
            }
        }
    }

    fn install_program(&mut self, program: Vec<Inst>) {
        let mut pc = 0;
        for inst in program {
            self.program_memory[pc] = inst;
            pc += 1;
        }
    }

    fn dump(&self) {
        println!("--- Emulation Dump");
        println!("Stepping, pc = {}, inst = {:?}, tick = {}", self.pc,
            self.program_memory[self.pc], self.tick);
        println!("Regs: {:?}", self.x);
        println!("Instructions: {:?}", &self.program_memory[0..32]);
        println!("Heap: {:?}", &self.heap[0..32]);
        // for x in &self.heap[0..32] {
        //     if *x == 1 {
        //         print!("{}", x);
        //     } else {
        //         print!(" ");
        //     }
        // }
        // println!();
        println!("---");
    }
}

fn main() {
    let mut cpu: CPU = CPU::default();
    cpu.install_program(vec![
    ]);
    cpu.run();
    cpu.dump();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci() {
        let mut cpu: CPU = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 0),       // a
            Inst::AddImmediate(1, 1),       // b
            Inst::AddImmediate(4, 15),      // limit
            Inst::Add(2, 0, 1),             // c = a + b
            Inst::Move(0, 1),               // a = b
            Inst::Move(1, 2),               // b = c
            Inst::AddImmediate(3, 1),       // i += 1
            Inst::BranchLessThan(3, 4, -5), // go up 4 if c < b
        ]);
        cpu.run();
        assert_eq!(cpu.x[1], 987);
    }

    #[test]
    fn test_memory() {
        let mut cpu: CPU = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 69), // val
            Inst::AddImmediate(1, 100), // addr
            Inst::StoreWord(0, 0, 1),
            Inst::LoadWord(2, 0, 1),
        ]);
        cpu.run();
        assert_eq!(cpu.x[2], 69);
    }

    #[test]
    fn test_rule110() {
        let mut cpu: CPU = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(2, 1),       // First 1
            Inst::AddImmediate(3, 0),       // Addr
            Inst::StoreWord(2, 0, 3),       // store
            Inst::AddImmediate(0, 32),      // limit
            Inst::AddImmediate(1, 1),       // start i from 1

            // Outer Loop Start
            Inst::Xor(4, 4, 4),             // set j = 0
            Inst::Xor(5, 5, 5),             // set left = 0
            Inst::Xor(6, 6, 6),             // set center = 0
            Inst::Xor(7, 7, 7),             // set right = 0
            Inst::LoadWord(6, 0, 4),

                // Inner Loop start
                Inst::Move(8, 4),
                Inst::AddImmediate(8, 1),
                Inst::LoadWord(7, 0, 8),

                Inst::Xor(9, 6, 5),
                Inst::XorImm(10, 7, 1),
                Inst::And(10, 5, 10),
                Inst::Or(11, 9, 10),
                Inst::StoreWord(11, 0, 4),

                Inst::Move(5, 6),
                Inst::Move(6, 7),
                Inst::AddImmediate(4, 1),       // j += 1
                Inst::BranchLessEq(4, 1, -12), // go up if j < i (size)

            Inst::AddImmediate(1, 1),       // i += 1
            Inst::BranchLessThan(1, 0, -19), // go up if i < limit
        ]);
        cpu.run();

        assert_eq!(&cpu.heap[0..32], [1, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1]);
    }

    #[test]
    fn test_and() {
        let mut cpu = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 1),
            Inst::AddImmediate(1, 0),
            Inst::And(3, 1, 1),
            Inst::And(4, 1, 0),
            Inst::And(5, 0, 1),
            Inst::And(6, 0, 0),
            Inst::AddImmediate(7, 5),
            Inst::AddImmediate(8, 2),
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
        let mut cpu = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 1),
            Inst::AddImmediate(1, 0),
            Inst::Or(3, 1, 1),
            Inst::Or(4, 1, 0),
            Inst::Or(5, 0, 1),
            Inst::Or(6, 0, 0),
            Inst::AddImmediate(7, 5),
            Inst::AddImmediate(8, 2),
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
        let mut cpu = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 1),
            Inst::AddImmediate(1, 0),
            Inst::Xor(3, 1, 1),
            Inst::Xor(4, 1, 0),
            Inst::Xor(5, 0, 1),
            Inst::Xor(6, 0, 0),
            Inst::AddImmediate(7, 5),
            Inst::AddImmediate(8, 3),
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
        let mut cpu = CPU::default();
        cpu.install_program(vec![
            Inst::AddImmediate(0, 1),
            Inst::AddImmediate(1, 0),
            Inst::Xor(3, 1, 0),
            Inst::Xor(4, 0, 0),
        ]);
        cpu.run();
        assert_eq!(cpu.x[3], 1);
        assert_eq!(cpu.x[4], 0);
    }
}
