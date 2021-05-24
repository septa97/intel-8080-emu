use std::fs::File;
use std::io;
use std::{env, io::Read};

fn disassemble(file_path: &String) -> Result<(), io::Error> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    let bytes = file.read_to_end(&mut buffer)?;

    println!("ROM size: {} bytes", bytes);

    let mut pc: usize = 0;

    while pc < buffer.len() {
        let opcode = buffer[pc];

        match opcode {
            0x00 | 0x08 | 0x10 | 0x18 | 0x28 | 0x38 | 0xCB | 0xD9 | 0xDD | 0xED | 0xFD => {
                println!("NOP");
                pc += 1;
            }
            0x01 => {
                println!("LXI   B,#${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x02 => {
                println!("STAX  B");
                pc += 1;
            }
            0x03 => {
                println!("INX   B");
                pc += 1;
            }
            0x04 => {
                println!("INR   B");
                pc += 1;
            }
            0x05 => {
                println!("DCR   B");
                pc += 1;
            }
            0x06 => {
                println!("MVI   B,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x07 => {
                println!("RLC");
                pc += 1;
            }
            0x09 => {
                println!("DAD   B");
                pc += 1;
            }
            0x0A => {
                println!("LDAX  B");
                pc += 1;
            }
            0x0B => {
                println!("DCX   B");
                pc += 1;
            }
            0x0C => {
                println!("INR   C");
                pc += 1;
            }
            0x0D => {
                println!("DCR   C");
                pc += 1;
            }
            0x0E => {
                println!("MVI   C,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x0F => {
                println!("RRC");
                pc += 1;
            }
            0x11 => {
                println!("LXI   D,#${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x12 => {
                println!("STAX  D");
                pc += 1;
            }
            0x13 => {
                println!("INX   D");
                pc += 1;
            }
            0x14 => {
                println!("INR   D");
                pc += 1;
            }
            0x15 => {
                println!("DCR   D");
                pc += 1;
            }
            0x16 => {
                println!("MVI   D,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x17 => {
                println!("RAL");
                pc += 1;
            }
            0x19 => {
                println!("DAD   D");
                pc += 1;
            }
            0x1A => {
                println!("LDAX  D");
                pc += 1;
            }
            0x1B => {
                println!("DCX   D");
                pc += 1;
            }
            0x1C => {
                println!("INR   E");
                pc += 1;
            }
            0x1D => {
                println!("DCR   E");
                pc += 1;
            }
            0x1E => {
                println!("MVI   E,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x1F => {
                println!("RAR");
                pc += 1;
            }
            0x20 => {
                println!("RIM");
                pc += 1;
            }
            0x21 => {
                println!("LXI   H,#${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x22 => {
                println!("SHLD  ${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x23 => {
                println!("INX   H");
                pc += 1;
            }
            0x24 => {
                println!("INR   H");
                pc += 1;
            }
            0x25 => {
                println!("DCR   H");
                pc += 1;
            }
            0x26 => {
                println!("MVI   H,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x27 => {
                println!("DAA");
                pc += 1;
            }
            0x29 => {
                println!("DAD   H");
                pc += 1;
            }
            0x2A => {
                println!("LHLD  ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x2B => {
                println!("DCX   H");
                pc += 1;
            }
            0x2C => {
                println!("INR   L");
                pc += 1;
            }
            0x2D => {
                println!("DCR   L");
                pc += 1;
            }
            0x2E => {
                println!("MVI   L,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x2F => {
                println!("CMA");
                pc += 1;
            }
            0x30 => {
                println!("SIM");
                pc += 1;
            }
            0x31 => {
                println!("LXI   SP,#${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x32 => {
                println!("STA   ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x33 => {
                println!("INX   SP");
                pc += 1;
            }
            0x34 => {
                println!("INR   M");
                pc += 1;
            }
            0x35 => {
                println!("DCR   M");
                pc += 1;
            }
            0x36 => {
                println!("MVI   M,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x37 => {
                println!("STC");
                pc += 1;
            }
            0x39 => {
                println!("DAD   SP");
                pc += 1;
            }
            0x3A => {
                println!("LDA   ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 3;
            }
            0x3B => {
                println!("DCX   SP");
                pc += 1;
            }
            0x3C => {
                println!("INR   A");
                pc += 1;
            }
            0x3D => {
                println!("DCR   A");
                pc += 1;
            }
            0x3E => {
                println!("MVI   A,#${:02x}", buffer[pc + 1]);
                pc += 2;
            }
            0x3F => {
                println!("CMC");
                pc += 1;
            }
            // TODO: continue here
            0xC3 => {
                println!("JMP   ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
                pc += 2;
            }
            _ => {
                // this is not accurate
                // it's possible that the skipped opcode use the next 1 or 2 bytes as data input for the instruction
                println!("Unknown opcode!");
                pc += 1;
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), io::Error> {
    if env::args().len() != 2 {
        panic!("usage: cargo run <path-to-ROM-file>");
    }

    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];

    disassemble(file_path)?;

    Ok(())
}
