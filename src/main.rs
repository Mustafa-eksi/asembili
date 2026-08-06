#![allow(dead_code)]

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
    Not(RegType, RegType, RegType),                     // reg (out), reg (in), reg (in)
    Xor(RegType, RegType, RegType),                     // reg (out), reg (in), reg (in)
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
            Inst::StoreWord(input, offset, addr) => {
                // TODO: expand array when needed
                self.heap[(addr as i64 + offset as i64) as usize] = self.x[input as usize];
            },
            Inst::LoadWord(reg, offset, addr) => {
                self.x[reg as usize] = self.heap[(addr as i64 + offset as i64) as usize];
            },
            _ => {todo!()}
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

    fn install_program(&mut self, program: Vec<Inst>) {
        let mut pc = 0;
        for inst in program {
            self.program_memory[pc] = inst;
            pc += 1;
        }
    }
}

fn main() {
    let mut cpu: CPU = CPU::default();
    cpu.install_program(vec![
    ]);
    cpu.run();
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
            // TODO
        ]);
        cpu.run();
        // assert_eq!(cpu.x[1], 987);
    }
}
