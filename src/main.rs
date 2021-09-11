use std::{env, process};

use crate::disassembler::disassemble;

mod disassembler;
mod state;

fn main() {
    if env::args().len() != 2 {
        panic!("usage: cargo run <path-to-ROM-file>");
    }

    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];

    if let Err(e) = disassemble(file_path) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
