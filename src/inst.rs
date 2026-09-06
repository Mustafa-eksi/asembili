use crate::program::RawInst;

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
    Compressed,

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

fn decompress(value: u16) -> Result<Inst, ()> {
    let opcode = value & 0x3;
    let funct3 = (value >> 13) & 0x7;
    let rd = ((value >> 7) & 0x1f) as RegType;
    let rs1 = rd;
    let rs2 = ((value >> 2) & 0x1f) as RegType;
    let rdp = ((value >> 2) & 0x7) as RegType;
    let rs1p = ((value >> 7) & 0x7) as RegType;
    let rs2p = ((value >> 2) & 0x7) as RegType;
    let uimm = ((value >> 12) & 0x1) << 5
        | ((value >> 11) & 0x1) << 4
        | ((value >> 10) & 0x1) << 3
        | ((value >> 6) & 0x1) << 2
        | ((value >> 5) & 0x1) << 6;
    let funct2 = (value >> 10) & 0x3;
    let funct4 = (value >> 12) & 0x1;
    let j_imm = ((value >> 12) & 0x1) << 11
        | ((value >> 11) & 0x1) << 4
        | ((value >> 9) & 0x3) << 8
        | ((value >> 8) & 0x1) << 10
        | ((value >> 7) & 0x1) << 6
        | ((value >> 6) & 0x1) << 7
        | ((value >> 3) & 0x7) << 1
        | ((value >> 2) & 0x1) << 5;
    let j_imm = ((j_imm as i32) << 20) >> 20;
    let cl_imm = ((value>>5)&0x3) | ((value>>10)&0x7)<<3;
    let ci_imm = (((value >> 12) & 0x1) << 5 | ((value >> 2) & 0x1f)) as i32;
    let ci_imm = (ci_imm << 26) >> 26;

    match (opcode, funct3, funct2, funct4, rd, rs2) {
        // Quadrant 0
        // FIXME: No check for illegal instruction nzuimm != 0
        (0, 0b000, _, _, _, _) => Ok(Inst::AddImmediate(8 + rdp, 2, ci_imm as i32)),
        (0, 0b010, _, _, _, _) => Ok(Inst::LoadWord(8 + rdp, cl_imm as i32, 8 + rs1p)),
        (0, 0b110, _, _, _, _) => Ok(Inst::StoreWord(8 + rs2p, uimm as i32, 8 + rs1p)),
        // Quadrant 1
        (1, 0b000, _, _, 0, _) => Ok(Inst::Nop),
        (1, 0b000, _, _, rd, _) => Ok(Inst::AddImmediate(rd, rd, ci_imm as i32)),
        (1, 0b001, _, _, _, _) => Ok(Inst::JumpAndLink(1, j_imm)),
        (1, 0b010, _, _, _, _) => {
            Ok(Inst::LoadImmediate(rd, ci_imm))
        }
        (1, 0b011, _, _, 2, _) => {
            let imm = ((value >> 12) & 0x1) << 9
                | ((value >> 6) & 0x1) << 4
                | ((value >> 5) & 0x1) << 6
                | ((value >> 3) & 0x3) << 7
                | ((value >> 2) & 0x1) << 5;
            let imm = ((imm as i32) << 22) >> 22;
            Ok(Inst::AddImmediate(2, 2, imm))
        },
        (1, 0b011, _, _, rd, _) => {
            let imm = (((value >> 12) & 0x1) << 5 | ((value >> 2) & 0x1f)) as i32;
            let imm = (imm << 26) >> 26;
            Ok(Inst::LoadUpperImmediate(rd, imm << 12))
        }
        (1, 0b100, f2, _, _, _) => {
            if f2 == 0b11 {
                let f2c = (value >> 5) & 0x3;
                match f2c {
                    0b00 => Ok(Inst::Subtract(8 + rs1p, 8 + rs1p, 8 + rs2p)),
                    0b01 => Ok(Inst::Xor(8 + rs1p, 8 + rs1p, 8 + rs2p)),
                    0b10 => Ok(Inst::Or(8 + rs1p, 8 + rs1p, 8 + rs2p)),
                    0b11 => Ok(Inst::And(8 + rs1p, 8 + rs1p, 8 + rs2p)),
                    _ => Err(()),
                }
            } else {
                let shamt = (((value >> 12) & 0x1) << 5 | ((value >> 2) & 0x1f)) as RegType;
                match f2 {
                    0b00 => Ok(Inst::ShiftRightLogical(8 + rs1p, 8 + rs1p, shamt)),
                    0b01 => Ok(Inst::ShiftRightArithmetic(8 + rs1p, 8 + rs1p, shamt)),
                    _ => Err(()),
                }
            }
        }
        (1, 0b101, _, _, _, _) => {
            let j_imm = ((value >> 12) & 0x1) << 11
                | ((value >> 11) & 0x1) << 4
                | ((value >> 9) & 0x3) << 8
                | ((value >> 8) & 0x1) << 10
                | ((value >> 7) & 0x1) << 6
                | ((value >> 6) & 0x1) << 7
                | ((value >> 3) & 0x7) << 1
                | ((value >> 2) & 0x1) << 5;
            let j_imm = ((j_imm as i32) << 20) >> 20;
            Ok(Inst::Jump(j_imm))
        }
        (1, 0b110, _, _, _, _) => {
            let offset = ((value >> 12) & 0x1) << 8
                | ((value >> 10) & 0x3) << 3
                | ((value >> 5) & 0x3) << 6
                | ((value >> 3) & 0x3) << 1
                | ((value >> 2) & 0x1) << 5;
            let offset = ((offset as i32) << 23) >> 23;
            Ok(Inst::BranchEquals(8 + rs1p, 0, offset))
        }
        (1, 0b111, _, _, _, _) => {
            let offset = ((value >> 12) & 0x1) << 8
                | ((value >> 10) & 0x3) << 3
                | ((value >> 5) & 0x3) << 6
                | ((value >> 3) & 0x3) << 1
                | ((value >> 2) & 0x1) << 5;
            let offset = ((offset as i32) << 23) >> 23;
            Ok(Inst::BranchNotEquals(8 + rs1p, 0, offset))
        }
        // Quadrant 2
        (2, 0b000, _, _, _, _) => {
            let shamt = (((value >> 12) & 0x1) << 5 | ((value >> 2) & 0x1f)) as RegType;
            Ok(Inst::ShiftLeftLogical(rd, rd, shamt))
        }
        (2, 0b010, _, _, _, _) => {
            let uimm = ((value >> 12) & 0x1) << 5
                | ((value >> 6) & 0x1) << 4
                | ((value >> 5) & 0x1) << 3
                | ((value >> 4) & 0x1) << 2
                | ((value >> 2) & 0x3) << 6;
            Ok(Inst::LoadWord(rd, uimm as i32, 2))
        }
        (2, 0b100, _, f4, _, _) => {
            if f4 == 0 {
                if rs2 == 0 {
                    Ok(Inst::JumpAndLinkReturn(0, rs1, 0))
                } else {
                    Ok(Inst::Move(rd, rs2))
                }
            } else {
                if rs2 == 0 {
                    if rs1 == 0 {
                        Ok(Inst::Ecall)
                    } else {
                        Ok(Inst::JumpAndLinkReturn(1, rs1, 0))
                    }
                } else {
                    Ok(Inst::Add(rd, rd, rs2))
                }
            }
        }
        (2, 0b110, _, _, _, _) => {
            let uimm = ((value >> 12) & 0x1) << 5
                | ((value >> 11) & 0x1) << 4
                | ((value >> 10) & 0x1) << 3
                | ((value >> 9) & 0x1) << 2
                | ((value >> 7) & 0x3) << 6;
            Ok(Inst::StoreWord(rs2, uimm as i32, 2))
        }
        _ => {
            eprintln!("Unknown compressed instruction.");
            Err(())
        }
    }
}

impl TryFrom<RawInst> for Inst {
    type Error = ();

    fn try_from(raw_inst: RawInst) -> Result<Self, Self::Error> {
        match raw_inst {
            RawInst::Compressed(c) => {decompress(c)},
            RawInst::Normal(n) => {Inst::try_from(n)}
        }
    }
}

impl TryFrom<u32> for Inst {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
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
