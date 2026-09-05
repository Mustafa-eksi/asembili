#![allow(dead_code)]

use std::path::Path;
use std::env;
use std::fs;
use goblin::error;

mod virtual_memory;
mod program;
mod inst;
mod cpu;
mod tests;
use cpu::Cpu;

fn main() -> error::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Insufficent arguments");
    }
    let path = Path::new(args[1].as_str());
    let buffer: Vec<u8> = fs::read(path)?;
    let mut cpu = Cpu::default();
    cpu.load_elf(&buffer);
    if args.into_iter().find(|x| x == "-d").is_some() {
        cpu.debug_mode();
    } else {
        cpu.run();
    }
    Ok(())
}
