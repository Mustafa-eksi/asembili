
const HEAP_SIZE: usize = 10;
const PROGRAM_SIZE: usize = 10;

// https://en.wikipedia.org/wiki/RISC-V_instruction_listings
// reg   = 1
// imm   = 1
#[derive(Debug, Copy, Clone, Default)]
enum Inst {
    #[default]
    Nop,
    LoadWord(u8, u8, u8),               // reg (out), imm, reg (in)
    StoreWord(u8, u8, u8),              // reg (out), imm, reg (in)
    LoadImmediate(u8, u8),              // reg (out), imm
    Move(u8, u8),                       // reg (out), reg (in)
    AddImmediate(u8, u8),               // reg (out), imm
    Add(u8, u8, u8),                    // reg (out), reg (in), reg (in)
    Subtract(u8, u8, u8),               // reg (out), reg (in), reg (in)
    Multiply(u8, u8, u8),               // reg (out), reg (in), reg (in)
    Divide(u8, u8, u8),                 // reg (out), reg (in), reg (in)
    Remainder(u8, u8, u8),              // reg (out), reg (in), reg (in)
    And(u8, u8, u8),                    // reg (out), reg (in), reg (in)
    Or(u8, u8, u8),                     // reg (out), reg (in), reg (in)
    Not(u8, u8, u8),                    // reg (out), reg (in), reg (in)
    Xor(u8, u8, u8),                    // reg (out), reg (in), reg (in)
    ShiftLeftLogical(u8, u8, u8),       // reg (out), reg (in), reg (in)
    ShiftRightLogical(u8, u8, u8),      // reg (out), reg (in), reg (in)
    ShiftRightArithmetic(u8, u8, u8),   // reg (out), reg (in), reg (in)
    Jump(u8),                           // imm
    BranchEquals(u8, u8, u8),           // reg, reg, imm
    BranchLessThan(u8, u8, u8),         // reg, reg, imm
    BranchLessEq(u8, u8, u8),           // reg, reg, imm
}

#[derive(Debug, Default)]
struct CPU {
    program_memory: [Inst; PROGRAM_SIZE],
    tick: u32, // tick counter
    pc: u32, // program counter
    ra: u32, // return address
    sp: u32, // stack pointer
    gp: u32, // global pointer
    // Skipping tp (thread pointer)
    fp: u32, // frame pointer
    x: [u32; 16], // general use registers
                    // TODO: introduce saved and temporary registers.
    heap: [u32; HEAP_SIZE],
}

impl CPU {
    fn run_inst(&mut self) {
        match self.program_memory[self.pc as usize] {
            Inst::AddImmediate(reg, imm) => {
                self.x[reg as usize] += imm as u32;
            },
            _ => {}
        }
    }

    fn step(&mut self) {
        self.tick += 1;
        self.run_inst();
        self.pc += 1;
    }
}

fn main() {
    let mut cpu: CPU = CPU::default();
    cpu.program_memory[0] = Inst::AddImmediate(0, 10);
    cpu.step();
    println!("{:?}", cpu);
}
