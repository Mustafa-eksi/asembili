


// https://en.wikipedia.org/wiki/RISC-V_instruction_listings
// reg   = 1
// imm   = 1 i32
type RegType = u8;
type ImmType = i32;
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum Inst {

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

fn decompress(value: u16) -> u32 {
    println!("Compressed instruction: {value:04x?}");
    0 as u32
}

impl TryFrom<u32> for Inst {
    type Error = ();

    fn try_from(mut value: u32) -> Result<Self, Self::Error> {
        if value & 0x3 != 3 {
            value = decompress((value<<16) as u16);
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
            _ => {
                eprintln!("Unknown instruction.");
                eprintln!("opcode: {opcode:08x?}");
                return Err(());
            },
        }
    }
}
