use std::io;
use std::{env, process};

use crate::disassembler::disassemble;
use crate::state::State8080;

mod disassembler;
mod state;

fn main() -> Result<(), io::Error> {
    if env::args().len() != 2 {
        panic!("usage: cargo run <path-to-ROM-file>");
    }

    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];

    // // for disassembly
    // if let Err(e) = disassemble(file_path) {
    //     eprintln!("Application error: {}", e);
    //     process::exit(1);
    // }

    let mut state: State8080 = Default::default();

    state.load_rom(file_path)?;
    state.init();

    loop {
        if state.halted() {
            break;
        }
        state.emulate_cycle();

        // FOR TESTING PURPOSES ONLY
        if state.pc() == 5 {
            if state.c() == 9 {
                let mut i = ((state.d() as usize) << 8) | (state.e() as usize);
                while (state.memory()[i] as char) != '$' {
                    print!("{}", state.memory()[i] as char);
                    i += 1;
                }
                print!("\n");
            } else if state.c() == 2 {
                println!("{}", state.e() as char);
            }
        }

        if state.pc() == 0 {
            println!("");
            break;
        }
    }

    Ok(())
}
