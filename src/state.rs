use std::io;
use std::{fs::File, io::Read};

pub const MEMORY_SIZE: usize = 65_536; // 2 ^ 16, 16-bit addresses

// on `z:1`, the `:1` is a bit field!
struct ConditionCodes {
    z: u8,
    s: u8,
    p: u8,
    cy: u8,
    ac: u8, // Space Invaders doesn't use this
    pad: u8,
}

pub struct State8080 {
    halted: bool,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16, // stack pointer
    pub pc: u16, // program counter
    memory: [u8; MEMORY_SIZE],
    cc: ConditionCodes,
    int_enable: u8, // TODO: figure out what this is
}

fn get_z(num: u8) -> u8 {
    if num == 0 {
        1
    } else {
        0
    }
}

fn get_s(num: u8) -> u8 {
    if num >> 7 == 1 {
        1
    } else {
        0
    }
}

fn get_p(num: u8) -> u8 {
    if num % 2 == 0 {
        1
    } else {
        0
    }
}

fn get_cy(has_overflowed: bool) -> u8 {
    if has_overflowed {
        1
    } else {
        0
    }
}

impl Default for State8080 {
    fn default() -> Self {
        State8080 {
            halted: false,
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            memory: [0; MEMORY_SIZE],
            cc: ConditionCodes {
                z: 0,
                s: 0,
                p: 0,
                cy: 0,
                ac: 0,
                pad: 0,
            },
            int_enable: 0,
        }
    }
}

impl State8080 {
    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn c(&self) -> u8 {
        self.c
    }

    pub fn d(&self) -> u8 {
        self.d
    }

    pub fn e(&self) -> u8 {
        self.e
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn memory(&self) -> [u8; MEMORY_SIZE] {
        self.memory
    }

    pub fn init(&mut self) {
        self.pc = 0x100;
        self.memory[5] = 0xC9;

        // TODO: For cpudiag.bin testing only
        // self.memory[0] = 0xC3;
        // self.memory[1] = 0x0;
        // self.memory[2] = 0x01;

        // self.memory[368] = 0x7;

        // self.memory[0x59C] = 0xC3;
        // self.memory[0x59D] = 0xC2;
        // self.memory[0x59E] = 0x05;

        // TODO: what initial value of SP
        // self.sp = 0xF000;
    }

    pub fn load_rom(&mut self, file_path: &String) -> Result<(), io::Error> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        // need `std::io::Read`
        let bytes = file.read_to_end(&mut buffer)?;

        println!("rom size: {} bytes", bytes);
        // TODO: maybe check if ROM size is too large to fit in memory?

        for i in 0..bytes {
            // is the offset (0x100 currently) correct?
            // TODO: I think we only need to use 0x100 for testing purposes
            self.memory[i + 0x100] = buffer[i];
        }

        Ok(())
    }

    // for ADD and ADI instructions
    fn add(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_add(rhs);

        // flags
        self.cc.z = get_z(ans);
        self.cc.s = get_s(ans);
        self.cc.p = get_p(ans & 0xFF);
        self.cc.cy = get_cy(has_overflowed);

        ans
    }

    // for ADC (and ACI) instructions
    fn adc(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_add(rhs);

        // flags
        self.cc.z = get_z(ans);
        self.cc.s = get_s(ans);
        self.cc.p = get_p(ans & 0xFF);
        self.cc.cy = get_cy(has_overflowed);

        ans + self.cc.cy // TODO: maybe this will overflow?
    }

    // for SUB and SUI instructions
    fn sub(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_sub(rhs);

        // flags
        self.cc.z = get_z(ans);
        self.cc.s = get_s(ans);
        self.cc.p = get_p(ans & 0xFF);
        self.cc.cy = get_cy(has_overflowed);

        ans
    }

    // for SBB (and SBI) instructions
    fn sbb(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_sub(rhs);

        // flags
        self.cc.z = get_z(ans);
        self.cc.s = get_s(ans);
        self.cc.p = get_p(ans & 0xFF);
        self.cc.cy = get_cy(has_overflowed);

        ans - self.cc.cy // TODO: maybe this will overflow?
    }

    // update `b` and `c`
    fn update_bc(&mut self, bc: u16) {
        self.c = bc as u8; // this should JUST truncate the higher byte
        self.b = (bc >> 8) as u8;
    }

    // update `d` and `e`
    fn update_de(&mut self, de: u16) {
        self.e = de as u8;
        self.d = (de >> 8) as u8;
    }

    // update `h` and `l`
    fn update_hl(&mut self, hl: u16) {
        self.l = hl as u8;
        self.h = (hl >> 8) as u8;
    }

    pub fn emulate_cycle(&mut self) {
        // fetch opcode
        let pc = self.pc as usize;
        let sp = self.sp as usize;
        let opcode = self.memory[pc];
        let bc = ((self.b as usize) << 8) | self.c as usize; // TODO: research more on `.into()`
        let de = ((self.d as usize) << 8) | self.e as usize;
        let hl = ((self.h as usize) << 8) | self.l as usize;

        let idx_pc_add1 = self.pc.wrapping_add(1) as usize;
        let idx_pc_add2 = self.pc.wrapping_add(2) as usize;

        let idx_sp_add1 = self.sp.wrapping_add(1) as usize;
        let idx_sp_sub1 = self.sp.wrapping_sub(1) as usize;
        let idx_sp_sub2 = self.sp.wrapping_sub(2) as usize;

        // FOR DEBUGGING PURPOSES ONLY
        println!("opcode: {:02x}, pc: {:04x}, sp: {:04x}, a: {:02x}, b: {:02x}, c: {:02x}, d: {:02x}, e: {:02x}, h: {:02x}, l: {:02x}, z: {}", opcode, self.pc, self.sp, self.a, self.b, self.c, self.d, self.e, self.h, self.l, self.cc.z);
        // END

        self.pc = self.pc.wrapping_add(1);

        match opcode {
            // ---- stack, I/O, and machine control group ----
            0x00 => (), // NOP
            // HLT
            0x76 => {
                // panic!("HLT was executed");
                self.halted = true;
            }
            // POP B
            0xC1 => {
                self.c = self.memory[sp];
                self.b = self.memory[idx_sp_add1];
                self.sp = self.sp.wrapping_add(2);
            }
            // PUSH B
            0xC5 => {
                self.memory[idx_sp_sub2] = self.c;
                self.memory[idx_sp_sub1] = self.b;
                self.sp = self.sp.wrapping_sub(2);
            }
            // POP D
            0xD1 => {
                self.e = self.memory[sp];
                self.d = self.memory[idx_sp_add1];
                self.sp = self.sp.wrapping_add(2);
            }
            0xD3 => (), // OUT d8 (special)
            // PUSH D
            0xD5 => {
                self.memory[idx_sp_sub2] = self.e;
                self.memory[idx_sp_sub1] = self.d;
                self.sp = self.sp.wrapping_sub(2);
            }
            0xDB => (), // IN d8 (special)
            // POP H
            0xE1 => {
                self.l = self.memory[sp];
                self.h = self.memory[idx_sp_add1];
                self.sp = self.sp.wrapping_add(2);
            }
            // XTHL (swap (SP) with HL)
            0xE3 => {
                let temp_high = self.memory[idx_sp_add1];
                let temp_low = self.memory[sp];
                self.memory[idx_sp_add1] = self.h;
                self.memory[sp] = self.l;
                self.h = temp_high;
                self.l = temp_low;
            }
            // PUSH H
            0xE5 => {
                self.memory[idx_sp_sub2] = self.l;
                self.memory[idx_sp_sub1] = self.h;
                self.sp = self.sp.wrapping_sub(2);
            }
            // POP PSW
            0xF1 => {
                let flags = self.memory[sp];

                self.cc.s = (flags & 0x80) >> 7;
                self.cc.z = (flags & 0x40) >> 6;
                self.cc.p = (flags & 0x04) >> 2;
                self.cc.cy = flags & 0x01;

                self.a = self.memory[idx_sp_add1];
                self.sp = self.sp.wrapping_add(2);
            }
            0xF3 => (), // DI (special)
            // PUSH PSW
            0xF5 => {
                let flags = (self.cc.s << 7) | (self.cc.z << 6) | (self.cc.p << 2) | (self.cc.cy) | 0x02;

                self.memory[idx_sp_sub2] = flags;
                self.memory[idx_sp_sub1] = self.a;
                self.sp = self.sp.wrapping_sub(2);
            }
            0xF9 => self.sp = hl as u16, // SPHL
            0xFB => (),                  // EI (special)
            // ---- illegal/undocumented group ----
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0xCB | 0xD9 | 0xDD | 0xED | 0xFD => (),
            // ---- data transfer group ----
            // LXI B,d16
            0x01 => {
                self.c = self.memory[idx_pc_add1];
                self.b = self.memory[idx_pc_add2];
                self.pc = self.pc.wrapping_add(2);
            }
            0x02 => self.memory[bc] = self.a, // STAX B
            // MVI B,d8
            0x06 => {
                self.b = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            0x0A => self.a = self.memory[bc], // LDAX B
            // MVI C,d8
            0x0E => {
                self.c = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            // LXI D,d16
            0x11 => {
                self.e = self.memory[idx_pc_add1];
                self.d = self.memory[idx_pc_add2];
                self.pc = self.pc.wrapping_add(2);
            }
            0x12 => self.memory[de] = self.a, // STAX D
            // MVI D,d8
            0x16 => {
                self.d = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            0x1A => self.a = self.memory[de], // LDAX D
            // MVI E,d8
            0x1E => {
                self.e = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            // LXI H,d16
            0x21 => {
                self.l = self.memory[idx_pc_add1];
                self.h = self.memory[idx_pc_add2];
                self.pc = self.pc.wrapping_add(2);
            }
            // SHLD a16
            0x22 => {
                self.memory[idx_pc_add1] = self.l;
                self.memory[idx_pc_add2] = self.h;
                self.pc = self.pc.wrapping_add(2);
            }
            // MVI H,d8
            0x26 => {
                self.h = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            // LHLD D
            0x2A => {
                self.l = self.memory[idx_pc_add1];
                self.h = self.memory[idx_pc_add2];
                self.pc = self.pc.wrapping_add(2);
            }
            // MVI L,d8
            0x2E => {
                self.l = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            // LXI SP,d16
            0x31 => {
                let value =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);
                self.sp = value;
                self.pc = self.pc.wrapping_add(2);
            }
            // STA a16
            0x32 => {
                let address = ((self.memory[idx_pc_add2] as usize) << 8) | (self.memory[idx_pc_add1] as usize);
                self.memory[address] = self.a;
                self.pc = self.pc.wrapping_add(2);
            }
            // MVI M,d8
            0x36 => {
                // `M` is memory location pointed by `HL` pair
                self.memory[hl] = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            // LDA a16
            0x3A => {
                let address = ((self.memory[idx_pc_add2] as usize) << 8) | (self.memory[idx_pc_add1] as usize);
                self.a = self.memory[address];
                self.pc = self.pc.wrapping_add(2);
            }
            // MVI A,d8
            0x3E => {
                self.a = self.memory[idx_pc_add1];
                self.pc = self.pc.wrapping_add(1);
            }
            0x40 => self.b = self.b, // MOV B,B (does this makes sense to implement???)
            0x41 => self.b = self.c, // MOV B,C
            0x42 => self.b = self.d, // MOV B,D
            0x43 => self.b = self.e, // MOV B,E
            0x44 => self.b = self.h, // MOV B,H
            0x45 => self.b = self.l, // MOV B,L
            0x46 => self.b = self.memory[hl], // MOV B,M (move to B the value at memory[HL])
            0x47 => self.b = self.a, // MOV B,A
            0x48 => self.c = self.b, // MOV C,B
            0x49 => self.c = self.c, // MOV C,C (does this makes sense to implement???)
            0x4A => self.c = self.d, // MOV C,D
            0x4B => self.c = self.e, // MOV C,E
            0x4C => self.c = self.h, // MOV C,H
            0x4D => self.c = self.l, // MOV C,L
            0x4E => self.c = self.memory[hl], // MOV C,M (move to C the value at memory[HL])
            0x4F => self.c = self.a, // MOV C,A
            0x50 => self.d = self.b, // MOV D,B
            0x51 => self.d = self.c, // MOV D,C
            0x52 => self.d = self.d, // MOV D,D (does this makes sense to implement???)
            0x53 => self.d = self.e, // MOV D,E
            0x54 => self.d = self.h, // MOV D,H
            0x55 => self.d = self.l, // MOV D,L
            0x56 => self.d = self.memory[hl], // MOV D,M (move to D the value at memory[HL])
            0x57 => self.d = self.a, // MOV D,A
            0x58 => self.e = self.b, // MOV E,B
            0x59 => self.e = self.c, // MOV E,C
            0x5A => self.e = self.d, // MOV E,D
            0x5B => self.e = self.e, // MOV E,E (does this makes sense to implement???)
            0x5C => self.e = self.h, // MOV E,H
            0x5D => self.e = self.l, // MOV E,L
            0x5E => self.e = self.memory[hl], // MOV E,M (move to E the value at memory[HL])
            0x5F => self.e = self.a, // MOV E,A
            0x60 => self.h = self.b, // MOV H,B
            0x61 => self.h = self.c, // MOV H,C
            0x62 => self.h = self.d, // MOV H,D
            0x63 => self.h = self.e, // MOV H,E
            0x64 => self.h = self.h, // MOV H,H (does this makes sense to implement???)
            0x65 => self.h = self.l, // MOV H,L
            0x66 => self.h = self.memory[hl], // MOV H,M (move to H the value at memory[HL])
            0x67 => self.h = self.a, // MOV H,A
            0x68 => self.l = self.b, // MOV L,B
            0x69 => self.l = self.c, // MOV L,C
            0x6A => self.l = self.d, // MOV L,D
            0x6B => self.l = self.e, // MOV L,E
            0x6C => self.l = self.h, // MOV L,H
            0x6D => self.l = self.l, // MOV L,L (does this makes sense to implement???)
            0x6E => self.l = self.memory[hl], // MOV L,M (move to L the value at memory[HL])
            0x6F => self.l = self.a, // MOV L,A
            0x70 => self.memory[hl] = self.b, // MOV M,B
            0x71 => self.memory[hl] = self.c, // MOV M,C
            0x72 => self.memory[hl] = self.d, // MOV M,D
            0x73 => self.memory[hl] = self.e, // MOV M,E
            0x74 => self.memory[hl] = self.h, // MOV M,H
            0x75 => self.memory[hl] = self.l, // MOV M,L
            0x77 => self.memory[hl] = self.a, // MOV M,A
            0x78 => self.a = self.b, // MOV A,B
            0x79 => self.a = self.c, // MOV A,C
            0x7A => self.a = self.d, // MOV A,D
            0x7B => self.a = self.e, // MOV A,E
            0x7C => self.a = self.h, // MOV A,H
            0x7D => self.a = self.l, // MOV A,L
            0x7E => self.a = self.memory[hl], // MOV A,M
            0x7F => self.a = self.a, // MOV A,A (does this makes sense to implement???)
            // XCHG (swap DE and HL)
            0xEB => {
                let temp_high = self.h;
                let temp_low = self.l;
                self.h = self.d;
                self.l = self.e;
                self.d = temp_high;
                self.e = temp_low;
            }
            // ---- arithmetic group ----
            // INX B
            0x03 => {
                let new_bc = (bc as u16).wrapping_add(1);

                self.update_bc(new_bc);
            }
            // INR B
            0x04 => {
                let new_b = self.b.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_b);
                self.cc.s = get_s(new_b);
                self.cc.p = get_p(new_b);

                self.b = new_b;
            }
            // DCR B
            0x05 => {
                let new_b = self.b.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_b);
                self.cc.s = get_s(new_b);
                self.cc.p = get_p(new_b);

                self.b = new_b;
            }
            // DAD B
            0x09 => {
                let (new_hl, has_overflowed) = (hl as u16).overflowing_add(bc as u16);

                // flags
                self.cc.cy = get_cy(has_overflowed);

                self.update_hl(new_hl);
            }
            // DCX B
            0x0B => {
                let new_bc = (bc as u16).wrapping_sub(1);
                self.c = new_bc as u8; // this should JUST truncate the higher byte
                self.b = (new_bc >> 8) as u8;
            }
            // INR C
            0x0C => {
                let new_c = self.c.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_c);
                self.cc.s = get_s(new_c);
                self.cc.p = get_p(new_c);

                self.c = new_c;
            }
            // DCR C
            0x0D => {
                let new_c = self.c.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_c);
                self.cc.s = get_s(new_c);
                self.cc.p = get_p(new_c);

                self.c = new_c;
            }
            // INX D
            0x13 => {
                let new_de = (de as u16).wrapping_add(1);

                self.update_de(new_de);
            }
            // INR D
            0x14 => {
                let new_d = self.d.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_d);
                self.cc.s = get_s(new_d);
                self.cc.p = get_p(new_d);

                self.d = new_d;
            }
            // DCR D
            0x15 => {
                let new_d = self.d.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_d);
                self.cc.s = get_s(new_d);
                self.cc.p = get_p(new_d);

                self.d = new_d;
            }
            // DAD D
            0x19 => {
                let (new_hl, has_overflowed) = (hl as u16).overflowing_add(de as u16);

                // flags
                self.cc.cy = get_cy(has_overflowed);

                self.update_hl(new_hl);
            }
            // DCX D
            0x1B => {
                let new_de = (de as u16).wrapping_sub(1);
                self.e = new_de as u8; // this should JUST truncate the higher byte
                self.d = (new_de >> 8) as u8;
            }
            // INR E
            0x1C => {
                let new_e = self.e.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_e);
                self.cc.s = get_s(new_e);
                self.cc.p = get_p(new_e);

                self.e = new_e;
            }
            // DCR E
            0x1D => {
                let new_e = self.e.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_e);
                self.cc.s = get_s(new_e);
                self.cc.p = get_p(new_e);

                self.e = new_e;
            }
            // INX H
            0x23 => {
                let new_hl = (hl as u16).wrapping_add(1);

                self.update_hl(new_hl);
            }
            // INR H
            0x24 => {
                let new_h = self.h.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_h);
                self.cc.s = get_s(new_h);
                self.cc.p = get_p(new_h);

                self.h = new_h;
            }
            // DCR H
            0x25 => {
                let new_h = self.h.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_h);
                self.cc.s = get_s(new_h);
                self.cc.p = get_p(new_h);

                self.h = new_h;
            }
            // DAD H
            0x29 => {
                let hl_u16 = hl as u16;
                let (new_hl, has_overflowed) = hl_u16.overflowing_add(hl_u16); // TODO: not sure if emulator101.com has a typo but I'm assuming it's `HL` and not `HI`

                // flags
                self.cc.cy = get_cy(has_overflowed);

                self.update_hl(new_hl);
            }
            // DCX H
            0x2B => {
                let new_hl = (hl as u16).wrapping_sub(1);
                self.l = new_hl as u8; // this should JUST truncate the higher byte
                self.h = (new_hl >> 8) as u8;
            }
            // INR L
            0x2C => {
                let new_l = self.l.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_l);
                self.cc.s = get_s(new_l);
                self.cc.p = get_p(new_l);

                self.l = new_l;
            }
            // DCR L
            0x2D => {
                let new_l = self.l.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_l);
                self.cc.s = get_s(new_l);
                self.cc.p = get_p(new_l);

                self.l = new_l;
            }
            // INX SP
            0x33 => {
                let new_sp = self.sp.wrapping_add(1);

                self.sp = new_sp;
            }
            // INR M
            0x34 => {
                let new_hl_mem = self.memory[hl].wrapping_add(1);

                // flags
                self.cc.z = get_z(new_hl_mem);
                self.cc.s = get_s(new_hl_mem);
                self.cc.p = get_p(new_hl_mem);

                self.memory[hl] = new_hl_mem;
            }
            // DCR M
            0x35 => {
                let new_hl_mem = self.memory[hl].wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_hl_mem);
                self.cc.s = get_s(new_hl_mem);
                self.cc.p = get_p(new_hl_mem);

                self.memory[hl] = new_hl_mem;
            }
            // DAD SP
            0x39 => {
                let (new_hl, has_overflowed) = (hl as u16).overflowing_add(self.sp); // TODO: verify if `SP` is correct

                // flags
                self.cc.cy = get_cy(has_overflowed);

                self.update_hl(new_hl);
            }
            // DCX SP
            0x3B => {
                let new_sp = self.sp.wrapping_sub(1);

                self.sp = new_sp;
            }
            // INR A
            0x3C => {
                let new_a = self.a.wrapping_add(1);

                // flags
                self.cc.z = get_z(new_a);
                self.cc.s = get_s(new_a);
                self.cc.p = get_p(new_a);

                self.a = new_a;
            }
            // DCR A
            0x3D => {
                let new_a = self.a.wrapping_sub(1);

                // flags
                self.cc.z = get_z(new_a);
                self.cc.s = get_s(new_a);
                self.cc.p = get_p(new_a);

                self.a = new_a;
            }
            0x80 => self.a = self.add(self.a, self.b), // ADD B
            0x81 => self.a = self.add(self.a, self.c), // ADD C
            0x82 => self.a = self.add(self.a, self.d), // ADD D
            0x83 => self.a = self.add(self.a, self.e), // ADD E
            0x84 => self.a = self.add(self.a, self.h), // ADD H
            0x85 => self.a = self.add(self.a, self.l), // ADD L
            0x86 => self.a = self.add(self.a, self.memory[hl]), // ADD M (A = A + (HL))
            0x87 => self.a = self.add(self.a, self.a), // ADD A (A = A + A)
            0x88 => self.a = self.adc(self.a, self.b), // ADC B
            0x89 => self.a = self.adc(self.a, self.c), // ADC C
            0x8A => self.a = self.adc(self.a, self.d), // ADC D
            0x8B => self.a = self.adc(self.a, self.e), // ADC E
            0x8C => self.a = self.adc(self.a, self.h), // ADC H
            0x8D => self.a = self.adc(self.a, self.l), // ADC L
            0x8E => self.a = self.adc(self.a, self.memory[hl]), // ADC M (A = A + (HL) + CY)
            0x8F => self.a = self.adc(self.a, self.a), // ADC A (A = A + A + CY)
            0xC6 => self.a = self.add(self.a, self.memory[idx_pc_add1]), // ADI D8 (rhs is an immediate value)
            0xCE => self.a = self.adc(self.a, self.memory[idx_pc_add1]), // ACI D8 (rhs is an immediate value PLUS the carry flag value)
            0x90 => self.a = self.sub(self.a, self.b),                   // SUB B
            0x91 => self.a = self.sub(self.a, self.c),                   // SUB C
            0x92 => self.a = self.sub(self.a, self.d),                   // SUB D
            0x93 => self.a = self.sub(self.a, self.e),                   // SUB E
            0x94 => self.a = self.sub(self.a, self.h),                   // SUB H
            0x95 => self.a = self.sub(self.a, self.l),                   // SUB L
            0x96 => self.a = self.sub(self.a, self.memory[hl]),          // SUB M (A = A - (HL))
            0x97 => self.a = self.sub(self.a, self.a),                   // SUB A (A = A - A)
            0x98 => self.a = self.sbb(self.a, self.b),                   // SBB B
            0x99 => self.a = self.sbb(self.a, self.c),                   // SBB C
            0x9A => self.a = self.sbb(self.a, self.d),                   // SBB D
            0x9B => self.a = self.sbb(self.a, self.e),                   // SBB E
            0x9C => self.a = self.sbb(self.a, self.h),                   // SBB H
            0x9D => self.a = self.sbb(self.a, self.l),                   // SBB L
            0x9E => self.a = self.sbb(self.a, self.memory[hl]), // SBB M (A = A - (HL) - CY)
            0x9F => self.a = self.sbb(self.a, self.a),          // SBB A (A = A - A - CY)
            0xD6 => self.a = self.sub(self.a, self.memory[idx_pc_add1]), // SUI D8 (rhs is an immediate value)
            0xDE => self.a = self.sbb(self.a, self.memory[idx_pc_add1]), // SBI D8 (rhs is an immediate value MINUS the carry flag value)
            // ---- logical group ----
            // RLC
            0x07 => {
                let bit_holder = self.a >> 7;
                self.a <<= 1;
                self.a |= bit_holder;

                self.cc.cy = bit_holder;
            }
            // RRC
            0x0F => {
                let bit_holder = self.a & 1;
                self.a >>= 1;
                self.a |= bit_holder << 7;

                self.cc.cy = bit_holder;
            }
            // RAL
            0x17 => {
                let bit_holder = self.a >> 7;
                self.a <<= 1;
                self.a |= self.cc.cy;

                self.cc.cy = bit_holder;
            }
            // RAR
            0x1F => {
                let bit_holder = self.a & 1;
                self.a >>= 1;
                self.a |= self.cc.cy << 7;

                self.cc.cy = bit_holder;
            }
            0x27 => (),                       // DAA (special)
            0x2F => self.a = !self.a,         // CMA
            0x37 => self.cc.cy = 1,           // STC
            0x3F => self.cc.cy = !self.cc.cy, // CMC
            0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xA6 | 0xA7 => {
                match opcode {
                    0xA0 => self.a &= self.b,          // ANA B
                    0xA1 => self.a &= self.c,          // ANA C
                    0xA2 => self.a &= self.d,          // ANA D
                    0xA3 => self.a &= self.e,          // ANA E
                    0xA4 => self.a &= self.h,          // ANA H
                    0xA5 => self.a &= self.l,          // ANA L
                    0xA6 => self.a &= self.memory[hl], // ANA M
                    0xA7 => self.a &= self.a,          // ANA A (does something happen with this???)
                    _ => panic!("This shouldn't be reached."),
                }

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);
            }
            0xA8 | 0xA9 | 0xAA | 0xAB | 0xAC | 0xAD | 0xAE | 0xAF => {
                match opcode {
                    0xA8 => self.a ^= self.b,          // XRA B
                    0xA9 => self.a ^= self.c,          // XRA C
                    0xAA => self.a ^= self.d,          // XRA D
                    0xAB => self.a ^= self.e,          // XRA E
                    0xAC => self.a ^= self.h,          // XRA H
                    0xAD => self.a ^= self.l,          // XRA L
                    0xAE => self.a ^= self.memory[hl], // XRA M
                    0xAF => self.a ^= self.a,          // XRA A (this is sure 0 right???)
                    _ => panic!("This shouldn't be reached."),
                }

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);
            }
            0xB0 | 0xB1 | 0xB2 | 0xB3 | 0xB4 | 0xB5 | 0xB6 | 0xB7 => {
                match opcode {
                    0xB0 => self.a |= self.b,          // ORA B
                    0xB1 => self.a |= self.c,          // ORA C
                    0xB2 => self.a |= self.d,          // ORA D
                    0xB3 => self.a |= self.e,          // ORA E
                    0xB4 => self.a |= self.h,          // ORA H
                    0xB5 => self.a |= self.l,          // ORA L
                    0xB6 => self.a |= self.memory[hl], // ORA M
                    0xB7 => self.a |= self.a,          // ORA A (does something happen with this???)
                    _ => panic!("This shouldn't be reached."),
                }

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);
            }
            0xB8 | 0xB9 | 0xBA | 0xBB | 0xBC | 0xBD | 0xBE | 0xBF => {
                let (result, has_overflowed) = match opcode {
                    0xB8 => self.a.overflowing_sub(self.b),          // CMP B
                    0xB9 => self.a.overflowing_sub(self.c),          // CMP C
                    0xBA => self.a.overflowing_sub(self.d),          // CMP D
                    0xBB => self.a.overflowing_sub(self.e),          // CMP E
                    0xBC => self.a.overflowing_sub(self.h),          // CMP H
                    0xBD => self.a.overflowing_sub(self.l),          // CMP L
                    0xBE => self.a.overflowing_sub(self.memory[hl]), // CMP M
                    0xBF => self.a.overflowing_sub(self.a), // CMP A (does something happen with this???)
                    _ => panic!("This shouldn't be reached."),
                };

                self.cc.z = get_z(result);
                self.cc.s = get_s(result);
                self.cc.p = get_p(result);
                self.cc.cy = get_cy(has_overflowed);
            }
            // TODO: maybe compress ANI, XRI, ORI, and CPI?
            // ANI d8
            0xE6 => {
                self.a &= self.memory[idx_pc_add1];

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);

                self.pc = self.pc.wrapping_add(1);
            }
            // XRI d8
            0xEE => {
                self.a ^= self.memory[idx_pc_add1];

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);

                self.pc = self.pc.wrapping_add(1);
            }
            // ORI d8
            0xF6 => {
                self.a |= self.memory[idx_pc_add1];

                self.cc.z = get_z(self.a);
                self.cc.s = get_s(self.a);
                self.cc.p = get_p(self.a);
                self.cc.cy = get_cy(false);

                self.pc = self.pc.wrapping_add(1);
            }
            // CPI d8
            0xFE => {
                let (result, has_overflowed) = self.a.overflowing_sub(self.memory[idx_pc_add1]);

                self.cc.z = get_z(result);
                self.cc.s = get_s(result);
                self.cc.p = get_p(result);
                self.cc.cy = get_cy(has_overflowed);

                self.pc = self.pc.wrapping_add(1);
            }
            // ---- branch group ----
            // RNZ (if Z is 0, meaning NOT Zero on the arg(see `get_z` function))
            0xC0 => {
                if self.cc.z == 0 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JNZ adr (if Z is 0, meaning Not Zero(see `get_z` function))
            0xC2 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.z == 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // JMP adr
            0xC3 => {
                // TODO: repetitive use of `address` on some instructions
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                self.pc = address;
            }
            // CNZ adr (if Z is NOT ZERO, call the address)
            0xC4 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.z == 1 {
                    self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = self.pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 0 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b000)
            0xC7 => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00000000;
            }
            // RZ (if Z is 1, meaning Zero on the arg(see `get_z` function))
            0xC8 => {
                if self.cc.z == 1 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // RET
            0xC9 => {
                let low = self.memory[sp] as u16;
                let high = (self.memory[idx_sp_add1] as u16) << 8;
                self.pc = high | low;
                self.sp = self.sp.wrapping_add(2);
            }
            // JZ adr (if Z is NOT 0, meaning Zero on the arg(see `get_z` function))
            0xCA => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.z != 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CZ adr (if Z is 0, call the address)
            0xCC => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.z == 0 {
                    let next_pc = self.pc + 2;
                    self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = next_pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CALL adr
            0xCD => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                let next_pc = self.pc + 2;
                self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                self.memory[idx_sp_sub2] = next_pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = address;
            }
            // RST 1 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b001)
            0xCF => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00001000;
            }
            // RNC (if CY is 0)
            0xD0 => {
                if self.cc.cy == 0 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JNC adr (if CY is cleared)
            0xD2 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.cy == 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CNC adr (if CY is ZERO, call the address)
            0xD4 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.cy == 0 {
                    let next_pc = self.pc + 2;
                    self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = next_pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 2 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b010)
            0xD7 => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00010000;
            }
            // RC (if CY is 1)
            0xD8 => {
                if self.cc.cy == 1 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JC adr (if CY is NOT cleared)
            0xDA => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.cy != 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CC adr (if CY is NOT ZERO, call the address)
            0xDC => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.cy == 1 {
                    let next_pc = self.pc + 2;
                    self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = next_pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 3 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b011)
            0xDF => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00011000;
            }
            // RPO (if P is 0, meaning odd)
            0xE0 => {
                if self.cc.p == 0 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JPO adr (if P is odd)
            0xE2 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.p == 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CPO adr (if P is 0, meaning odd, call the address)
            0xE4 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.p == 0 {
                    // TODO: maybe initialize `next_pc` at the top to avoid redundant code?
                    // TODO: use a separate `push` and `pop` method for the stack
                    let next_pc = self.pc + 2;
                    self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = next_pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 4 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b100)
            0xE7 => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00100000;
            }
            // RPE (if P is 1, meaning even)
            0xE8 => {
                if self.cc.p == 1 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // PCHL
            0xE9 => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = hl as u16;
            }
            // JPE adr (if P is even)
            0xEA => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.p == 1 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CPE adr (if P is 1, meaning even, call the address)
            0xEC => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.p == 1 {
                    let next_pc = self.pc + 2;
                    self.memory[idx_sp_sub1] = (next_pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = next_pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 5 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b101)
            0xEF => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00101000;
            }
            // RP (if S is 0, meaning positive)
            0xF0 => {
                if self.cc.s == 0 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JP adr (jump if positive)
            0xF2 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                // if S is 0, meaning positive
                if self.cc.s == 0 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CP adr (if S is 0, meaning positive, call the address)
            0xF4 => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.s == 0 {
                    self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = self.pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 6 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b110)
            0xF7 => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00110000;
            }
            // RM (if S is 1, meaning Minus/negative)
            0xF8 => {
                if self.cc.s == 1 {
                    let low = self.memory[sp] as u16;
                    let high = (self.memory[idx_sp_add1] as u16) << 8;
                    self.pc = high | low;
                    self.sp = self.sp.wrapping_add(2);
                }
            }
            // JM adr (jump if minus/negative)
            0xFA => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                // if S is 1, meaning negative
                if self.cc.s == 1 {
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // CM adr (if S is 1, meaning Minus/negative, call the address)
            0xFC => {
                let address =
                    ((self.memory[idx_pc_add2] as u16) << 8) | (self.memory[idx_pc_add1] as u16);

                if self.cc.s == 1 {
                    self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                    self.memory[idx_sp_sub2] = self.pc as u8;
                    self.sp = self.sp.wrapping_sub(2);
                    self.pc = address;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // RST 7 (call at address 0b00---000, where --- are values from 0b000 to 0b111, in this case is 0b111)
            0xFF => {
                self.memory[idx_sp_sub1] = (self.pc >> 8) as u8;
                self.memory[idx_sp_sub2] = self.pc as u8;
                self.sp = self.sp.wrapping_sub(2);
                self.pc = 0b00111000;
            } // _ => panic!("Unknown opcode!"), // TODO: uncomment to determine the unimplemented opcodes
        }
    }
}
