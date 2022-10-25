extern crate derive_more;

use std::mem;
use derive_more::Constructor;
use bitmatch::bitmatch;

use crate::asm::AsmExpr;
use crate::encoding::{ModRmAddressingMode, ModRmByte, RexPrefix};
use crate::prelude::*;

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct MoveRegToReg {
    pub source: RegView,
    pub dest: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct MoveImmToReg {
    pub imm: usize,
    pub dest: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct MoveImmToRegMemOffset {
    pub imm: usize,
    pub offset: isize,
    pub reg_to_deref: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct AddRegToReg {
    pub augend: RegView,
    pub addend: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct SubRegFromReg {
    pub minuend: RegView,
    pub subtrahend: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct MulRegByReg {
    pub multiplicand: RegView,
    pub multiplier: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct DivRegByReg {
    pub dividend: RegView,
    pub divisor: RegView,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct CompareImmWithReg {
    pub imm: usize,
    pub reg: RegView,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instr {
    // Assembly meta directives
    DirectiveSetCurrentSection(String),
    DirectiveDeclareGlobalSymbol(String),
    DirectiveDeclareLabel(String),
    DirectiveEmbedAscii(String),
    DirectiveEmbedU32(u32),
    DirectiveEqu(String, AsmExpr),

    // Instructions
    Return,
    PushFromReg(RegView),
    PopIntoReg(RegView),
    MoveRegToReg(MoveRegToReg),
    MoveImmToReg(MoveImmToReg),
    MoveImmToRegMemOffset(MoveImmToRegMemOffset),
    NegateRegister(Register),
    AddRegToReg(AddRegToReg),
    SubRegFromReg(SubRegFromReg),
    MulRegByReg(MulRegByReg),
    DivRegByReg(DivRegByReg),
    JumpToLabel(String),
    JumpToLabelIfEqual(String),
    CompareImmWithReg(CompareImmWithReg),
    Interrupt(u8),

    // TODO(PT): How to reintroduce this? Move a .equ symbol into a register
    //MoveSymbolToReg(MoveSymToReg),
}

impl Instr {
    pub fn render(&self) -> String {
        match self {
            Instr::Return => "ret".into(),
            Instr::PushFromReg(src) => format!("push %{}", src.asm_name()),
            Instr::PopIntoReg(dst) => format!("pop %{}", dst.asm_name()),
            Instr::MoveRegToReg(MoveRegToReg { source, dest }) => {
                format!("mov %{}, %{}", source.asm_name(), dest.asm_name())
            }
            Instr::MoveImmToReg(MoveImmToReg { imm, dest }) => {
                format!("mov $0x{imm:x}, %{}", dest.asm_name())
            }
            Instr::DirectiveDeclareGlobalSymbol(symbol_name) => {
                format!(".global {symbol_name}")
            }
            Instr::DirectiveDeclareLabel(label_name) => format!("{label_name}:"),
            Instr::NegateRegister(reg) => format!("neg %{}", reg.asm_name()),
            Instr::AddRegToReg(AddRegToReg { augend, addend }) => {
                format!("add %{}, %{}", augend.asm_name(), addend.asm_name())
            }
            Instr::DirectiveSetCurrentSection(section_name) => {
                format!(".section {section_name}")
            }
            _ => todo!("Instr.render() {self:?}"),
        }
    }

    pub fn assemble(&self) -> Vec<u8> {
        match self {
            Instr::PushFromReg(reg) => {
                vec![
                    0xff,
                    ModRmByte::with_opcode_extension(ModRmAddressingMode::RegisterDirect, 6, *reg),
                ]
            },
            Instr::PopIntoReg(reg) => {
                vec![
                    0x8f,
                    ModRmByte::with_opcode_extension(ModRmAddressingMode::RegisterDirect, 0, *reg),
                ]
            }
            Instr::MoveRegToReg(MoveRegToReg { source, dest }) => {
                vec![
                    RexPrefix::for_64bit_operand(),
                    0x89,
                    ModRmByte::from(ModRmAddressingMode::RegisterDirect, dest.0, Some(source.0))
                ]
            }
            Instr::MoveImmToReg(MoveImmToReg { imm, dest }) => {
                if dest.1 == AccessType::RX {
                    // MOV r64, imm64
                    let mut out = vec![];
                    out.push(RexPrefix::for_64bit_operand());
                    out.push((0xb8 + ModRmByte::register_index(dest.0)) as u8);
                    out.append(&mut (*imm as u64).to_le_bytes().to_vec());
                    return out;
                }

                let mut out = vec![];
                let mut imm_bytes = match dest.1 {
                    AccessType::RX | AccessType::EX => {
                        (*imm as u32).to_le_bytes().to_vec()
                    }
                    _ => todo!()
                };

                out.append(&mut vec![
                    // MOV r/m64, imm32
                    0xc7,
                    // Destination register
                    ModRmByte::with_opcode_extension(ModRmAddressingMode::RegisterDirect, 0, *dest),
                ]);
                out.append(&mut imm_bytes);
                out
            }
            Instr::AddRegToReg(AddRegToReg { augend, addend }) => {
                vec![
                    RexPrefix::for_64bit_operand(),
                    0x01,
                    ModRmByte::from(ModRmAddressingMode::RegisterDirect, augend.0, Some(addend.0)),
                ]
            }
            Instr::Return => {
                vec![
                    0xc3,
                ]
            }
            _ => todo!(),
        }
    }

    pub fn assembled_len(&self) -> usize {
        match self {
            Instr::PushFromReg(_) => 2,
            Instr::PopIntoReg(_) => 2,
            Instr::MoveRegToReg(_) => 3,
            Instr::MoveImmToReg(MoveImmToReg { imm, dest }) => {
                match dest.1 {
                    AccessType::RX => 10,
                    AccessType::EX => 6,
                    _ => todo!(),
                }
            },
            Instr::AddRegToReg(_) => 3,
            Instr::Return => 1,
            _ => todo!(),
        }
    }
}

pub trait InstrBytecodeProvider {
    fn get_byte(&self, offset: u64) -> u8;
}

pub struct InstrDisassembler<'a> {
    instr_bytecode_provider: &'a dyn InstrBytecodeProvider,
    cursor: usize,
    operand_size: AccessType,
}

impl<'a> InstrDisassembler<'a> {
    pub fn new(instr_bytecode_provider: &'a dyn InstrBytecodeProvider) -> Self {
        Self {
            instr_bytecode_provider,
            cursor: 0,
            operand_size: AccessType::EX,
        }
    }

    fn get_byte(&mut self) -> u8 {
        let ret = self.instr_bytecode_provider.get_byte(self.cursor as _);
        self.cursor += 1;
        ret
    }

    fn get_u64(&mut self) -> u64 {
        // Little-endian encoded directly at the cursor
        let mut bytes = vec![];
        for _ in 0..mem::size_of::<u64>() {
            bytes.push(self.get_byte());
        }
        let as_array: [u8; mem::size_of::<u64>()] = bytes.try_into().unwrap();
        u64::from_le_bytes(as_array)
    }

    fn get_u32(&mut self) -> u32 {
        // Little-endian encoded directly at the cursor
        let mut bytes = vec![];
        for _ in 0..mem::size_of::<u32>() {
            bytes.push(self.get_byte());
        }
        let as_array: [u8; mem::size_of::<u32>()] = bytes.try_into().unwrap();
        u32::from_le_bytes(as_array)
    }

    fn get_modrm_opcode_and_reg(&mut self) -> (u8, RegView) {
        let mod_rm_byte = self.get_byte();
        let opcode_extension = ModRmByte::get_opcode_extension(mod_rm_byte);
        let reg = RegView(ModRmByte::get_reg(mod_rm_byte), self.operand_size);
        (opcode_extension, reg)
    }

    fn get_modrm_regs(&mut self) -> (RegView, RegView) {
        let mod_rm_byte = self.get_byte();
        let (dst, src) = ModRmByte::get_regs(mod_rm_byte);
        (RegView(dst, self.operand_size), RegView(src, self.operand_size))
    }

    fn yield_seq_instr(&self, instr: Instr) -> InstrInfo {
        InstrInfo::seq(instr, self.cursor)
    }

    fn yield_jump_instr(&self, instr: Instr) -> InstrInfo {
        InstrInfo::jump(instr, self.cursor)
    }

    #[bitmatch]
    pub fn disassemble(&mut self) -> InstrInfo {
        let mut instr_byte = self.get_byte();

        // Look for REX.W prefix
        if instr_byte == 0x48 {
            // Consume the REX.W prefix
            instr_byte = self.get_byte();
            // TODO(PT): Here we can flesh out support for different operand sizes
            self.operand_size = AccessType::RX;
        }

        // Instructions that are matched directly by opcode
        let maybe_instr_info = match instr_byte {
            0x01 => {
                // ADD r/m64, r64
                let (augend, addend) = self.get_modrm_regs();
                Some(self.yield_seq_instr(Instr::AddRegToReg(AddRegToReg::new(augend, addend))))
            }
            0x89 => {
                // MOV r/m64,r64
                let (dst, src) = self.get_modrm_regs();
                Some(self.yield_seq_instr(Instr::MoveRegToReg(MoveRegToReg::new(src, dst))))
            }
            0x8f => {
                // TODO(PT): Assume 64bit reg size for now, how to determine?
                // Check in an assembler
                self.operand_size = AccessType::RX;
                let (opcode_extension, reg) = self.get_modrm_opcode_and_reg();
                match opcode_extension {
                    0 => {
                        // POP r/m64
                        Some(self.yield_seq_instr(Instr::PopIntoReg(reg)))
                    }
                    _ => panic!("Unhandled opcode sequence: 8f /{opcode_extension}"),
                }
            }
            0xc3 => {
                Some(self.yield_jump_instr(Instr::Return))
            }
            0xc7 => {
                let (opcode_extension, reg) = self.get_modrm_opcode_and_reg();
                match opcode_extension {
                    0 => {
                        // C7 /0 iw
                        // MOV r/m64, imm32
                        let imm = self.get_u32();
                        Some(self.yield_seq_instr(Instr::MoveImmToReg(MoveImmToReg::new(imm as usize, reg))))
                    }
                    _ => panic!("Unhandled opcode sequence: c7 /{opcode_extension}"),
                }
            }
            0xff => {
                // TODO(PT): Assume 64bit reg size for now, how to determine?
                // Check in an assembler
                self.operand_size = AccessType::RX;
                let (opcode_extension, reg) = self.get_modrm_opcode_and_reg();
                match opcode_extension {
                    6 => {
                        // PUSH r/m[16|32|64]
                        Some(self.yield_seq_instr(Instr::PushFromReg(reg)))
                    }
                    _ => panic!("Unhandled opcode sequence: ff /{opcode_extension}"),
                }
            }
            // Handled down below
            _ => None,
        };
        if let Some(instr_info) = maybe_instr_info {
            // Instruction interpreted by direct opcode match
            return instr_info;
        }

        // Instructions/instruction families that need to be bitmatched
        #[bitmatch]
        match instr_byte {
            "10111iii" => {
                // B8+ rd id
                // MOV r64, imm64
                let dest_reg = ModRmByte::index_to_register(i);
                match self.operand_size {
                    AccessType::RX => {
                        let imm = self.get_u64();
                        self.yield_seq_instr(Instr::MoveImmToReg(MoveImmToReg::new(imm as usize, RegView(dest_reg, self.operand_size))))
                    }
                    _ => todo!("Unhandled access size"),
                }
            }
            _ => todo!("Unhandled opcode 0x{instr_byte:x}"),
        }
    }
}

#[derive(Debug)]
pub struct InstrInfo {
    pub instr: Instr,
    pub instr_size: usize,
    pub rip_increment: Option<usize>,
    pub jumped: bool,
}

impl InstrInfo {
    fn seq(instr: Instr, instr_size: usize) -> Self {
        Self {
            instr,
            instr_size,
            rip_increment: Some(instr_size),
            jumped: false,
        }
    }

    fn jump(instr: Instr, instr_size: usize) -> Self {
        Self {
            instr,
            instr_size,
            // Don't increment pc because the instruction will modify pc directly
            rip_increment: None,
            jumped: true,
        }
    }
}

mod test {
    use assert_hex::assert_eq_hex;

    use crate::instructions::{AddRegToReg, Instr, InstrBytecodeProvider, InstrDisassembler, MoveImmToReg, MoveRegToReg};
    use crate::prelude::RegView;

    impl InstrBytecodeProvider for Vec<u8> {
        fn get_byte(&self, offset: u64) -> u8 {
            self[offset as usize]
        }
    }

    fn validate_assembly_and_disassembly(instr_and_bytecode_pairs: Vec<(Instr, Vec<u8>)>) {
        for (instr, bytecode) in instr_and_bytecode_pairs.iter() {
            // Ensure the assembled instruction matches the expected bytecode
            let assembled = instr.assemble();
            println!("Validating that {instr:?} assembles correctly...");
            assert_eq_hex!(assembled, *bytecode);
            assert_eq!(assembled.len(), instr.assembled_len());

            // Ensure the disassembled bytecode matches the expected instruction
            println!("Validating that {instr:?} disassembles correctly...");
            let mut disassembler = InstrDisassembler::new(bytecode);
            let disassembled_instr_info = disassembler.disassemble();
            assert_eq!(disassembled_instr_info.instr, *instr);

        }
    }

    #[test]
    fn test_move_imm_to_reg() {
        validate_assembly_and_disassembly(vec![
            (Instr::MoveImmToReg(MoveImmToReg::new(0xcafe_babe_dead_beef, RegView::rsp())), vec![0x48, 0xbc, 0xef, 0xbe, 0xad, 0xde, 0xbe, 0xba, 0xfe, 0xca]),
            (Instr::MoveImmToReg(MoveImmToReg::new(5, RegView::rax())), vec![0x48, 0xb8, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            (Instr::MoveImmToReg(MoveImmToReg::new(0, RegView::eax())), vec![0xc7, 0xc0, 0x00, 0x00, 0x00, 0x00]),
        ]);
    }

    #[test]
    fn test_move_reg_to_reg() {
        validate_assembly_and_disassembly(vec![
            (Instr::MoveRegToReg(MoveRegToReg::new(RegView::rax(), RegView::rsp())), vec![0x48, 0x89, 0xc4]),
            (Instr::MoveRegToReg(MoveRegToReg::new(RegView::rcx(), RegView::rdx())), vec![0x48, 0x89, 0xca]),
        ]);
    }

    #[test]
    fn test_push() {
        validate_assembly_and_disassembly(vec![
            (Instr::PushFromReg(RegView::rax()), vec![0xff, 0xf0]),
            (Instr::PushFromReg(RegView::rdx()), vec![0xff, 0xf2]),
            (Instr::PushFromReg(RegView::rsp()), vec![0xff, 0xf4]),
        ]);
    }

    #[test]
    fn test_pop() {
        validate_assembly_and_disassembly(vec![
            (Instr::PopIntoReg(RegView::rax()), vec![0x8f, 0xc0]),
            (Instr::PopIntoReg(RegView::rdx()), vec![0x8f, 0xc2]),
            (Instr::PopIntoReg(RegView::rsp()), vec![0x8f, 0xc4]),
        ]);
    }

    #[test]
    fn test_add_reg_to_reg() {
        validate_assembly_and_disassembly(vec![
            (Instr::AddRegToReg(AddRegToReg::new(RegView::rax(), RegView::rsp())), vec![0x48, 0x01, 0xe0]),
            (Instr::AddRegToReg(AddRegToReg::new(RegView::rbx(), RegView::rbp())), vec![0x48, 0x01, 0xeb]),
            (Instr::AddRegToReg(AddRegToReg::new(RegView::rcx(), RegView::rcx())), vec![0x48, 0x01, 0xc9]),
        ]);
    }

    #[test]
    fn test_return() {
        validate_assembly_and_disassembly(vec![
            (Instr::Return, vec![0xc3]),
        ]);
    }
}
