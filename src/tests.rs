
#[cfg(test)]
mod tests {
    use crate::inst::*;
    use crate::cpu::*;

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

    // #[test]
    // fn decodes_riscv32c_compressed_instructions() {
    //     use crate::program::RawInst;
    //     use crate::inst::Inst;
    //     // (encoding, expected) -- encodings taken from the RISC-V "C" spec
    //     let cases: [(u16, Inst); 26] = [
    //         (0x1024, Inst::AddImmediate(9, 2, 40)),              // C.ADDI4SPN x9, 40
    //         (0x5504, Inst::LoadWord(9, 40, 10)),                 // C.LW x9, 40(x10)
    //         (0xd504, Inst::StoreWord(9, 40, 10)),                // C.SW x9, 40(x10)
    //         (0x10fd, Inst::AddImmediate(1, 1, -1)),              // C.ADDI x1, -1
    //         (0x0095, Inst::AddImmediate(1, 1, 5)),               // C.ADDI x1, 5
    //         (0x0001, Inst::Nop),                                 // C.NOP
    //         (0x2011, Inst::JumpAndLink(1, 4)),                   // C.JAL 4
    //         (0x3ff5, Inst::JumpAndLink(1, -4)),                  // C.JAL -4
    //         (0x50ed, Inst::LoadImmediate(1, -5)),                // C.LI x1, -5
    //         (0x6085, Inst::LoadUpperImmediate(1, 0x1000)),       // C.LUI x1, 1
    //         (0x8085, Inst::ShiftRightLogical(9, 9, 1)),          // C.SRLI x9, 1
    //         (0x8485, Inst::ShiftRightArithmetic(9, 9, 1)),       // C.SRAI x9, 1
    //         (0x8c89, Inst::Subtract(9, 9, 10)),                  // C.SUB x9, x10
    //         (0x8ca9, Inst::Xor(9, 9, 10)),                       // C.XOR x9, x10
    //         (0x8cc9, Inst::Or(9, 9, 10)),                        // C.OR x9, x10
    //         (0x8ce9, Inst::And(9, 9, 10)),                       // C.AND x9, x10
    //         (0x2021, Inst::Jump(8)),                             // C.J 8
    //         (0xc481, Inst::BranchEquals(9, 0, 8)),               // C.BEQZ x9, 8
    //         (0xfce5, Inst::BranchNotEquals(9, 0, -8)),           // C.BNEZ x9, -8
    //         (0x008a, Inst::ShiftLeftLogical(1, 1, 2)),           // C.SLLI x1, 2
    //         (0x50a2, Inst::LoadWord(1, 40, 2)),                  // C.LWSP x1, 40(x2)
    //         (0x8402, Inst::JumpAndLinkReturn(0, 8, 0)),          // C.JR x8
    //         (0x831e, Inst::Move(6, 7)),                          // C.MV x6, x7
    //         (0x9402, Inst::JumpAndLinkReturn(1, 8, 0)),          // C.JALR x8
    //         (0x931e, Inst::Add(6, 6, 7)),                        // C.ADD x6, x7
    //         (0xd406, Inst::StoreWord(1, 40, 2)),                 // C.SWSP x1, 40(x2)
    //     ];
    //     for (enc, expected) in cases {
    //         assert_eq!(
    //             Inst::try_from(RawInst::Compressed(enc)),
    //             Ok(expected),
    //             "Mismatch for encoding 0x{enc:04x}"
    //         );
    //     }
    // }

}
