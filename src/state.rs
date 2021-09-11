const MEMORY_SIZE: usize = 65_536; // 2 ^ 16, 16-bit addresses

// TODO: default values? check emulator101.com code for more info on `z:1`, etc.
struct ConditionCodes {
    z: u8,
    s: u8,
    p: u8,
    cy: u8,
    ac: u8, // Space Invaders doesn't use this
    pad: u8,
}

struct State8080 {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16, // stack pointer
    pc: u16, // program counter
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

impl State8080 {
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

    fn emulate_cycle(&mut self) {
        // fetch opcode
        let pc = self.pc as usize;
        let opcode = self.memory[pc];
        let bc = ((self.b as usize) << 8) | self.c as usize; // TODO: research more on `.into()`
        let de = ((self.d as usize) << 8) | self.e as usize;
        let hl = ((self.h as usize) << 8) | self.l as usize;

        match opcode {
            // NOP
            0x00 | 0x08 | 0x10 | 0x18 | 0x28 | 0x38 | 0xCB | 0xD9 | 0xDD | 0xED | 0xFD => (),
            // LXI B,D16
            0x01 => {
                self.c = self.memory[pc + 1];
                self.b = self.memory[pc + 2];
                self.pc += 2;
            }
            // TODO: maybe separate this from this group? `assignment group` maybe or something like that?
            // STAX B
            0x02 => self.memory[bc] = self.a,
            // INX B
            0x03 => {
                let (new_bc, _) = (bc as u16).overflowing_add(1);

                self.update_bc(new_bc);
            }
            // INR B
            0x04 => {
                let (new_b, _) = self.b.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_b);
                self.cc.s = get_s(new_b);
                self.cc.p = get_p(new_b);

                self.b = new_b;
            }
            // DCR B
            0x05 => {
                let (new_b, _) = self.b.overflowing_sub(1);

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
                let new_bc = (bc as u16) - 1;
                self.c = new_bc as u8; // this should JUST truncate the higher byte
                self.b = (new_bc >> 8) as u8;
            }
            // INR C
            0x0C => {
                let (new_c, _) = self.c.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_c);
                self.cc.s = get_s(new_c);
                self.cc.p = get_p(new_c);

                self.c = new_c;
            }
            // DCR C
            0x0D => {
                let (new_c, _) = self.c.overflowing_sub(1);

                // flags
                self.cc.z = get_z(new_c);
                self.cc.s = get_s(new_c);
                self.cc.p = get_p(new_c);

                self.c = new_c;
            }
            // INX D
            0x13 => {
                let (new_de, _) = (de as u16).overflowing_add(1);

                self.update_de(new_de);
            }
            // INR D
            0x14 => {
                let (new_d, _) = self.d.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_d);
                self.cc.s = get_s(new_d);
                self.cc.p = get_p(new_d);

                self.d = new_d;
            }
            // DCR D
            0x15 => {
                let (new_d, _) = self.d.overflowing_sub(1);

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
                let new_de = (de as u16) - 1;
                self.e = new_de as u8; // this should JUST truncate the higher byte
                self.d = (new_de >> 8) as u8;
            }
            // INR E
            0x1C => {
                let (new_e, _) = self.e.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_e);
                self.cc.s = get_s(new_e);
                self.cc.p = get_p(new_e);

                self.e = new_e;
            }
            // DCR E
            0x1D => {
                let (new_e, _) = self.e.overflowing_sub(1);

                // flags
                self.cc.z = get_z(new_e);
                self.cc.s = get_s(new_e);
                self.cc.p = get_p(new_e);

                self.e = new_e;
            }
            // INX H
            0x23 => {
                let (new_hl, _) = (hl as u16).overflowing_add(1);

                self.update_hl(new_hl);
            }
            // INR H
            0x24 => {
                let (new_h, _) = self.h.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_h);
                self.cc.s = get_s(new_h);
                self.cc.p = get_p(new_h);

                self.h = new_h;
            }
            // DCR H
            0x25 => {
                let (new_h, _) = self.h.overflowing_sub(1);

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
                let new_hl = (hl as u16) - 1;
                self.l = new_hl as u8; // this should JUST truncate the higher byte
                self.h = (new_hl >> 8) as u8;
            }
            // INR L
            0x2C => {
                let (new_l, _) = self.l.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_l);
                self.cc.s = get_s(new_l);
                self.cc.p = get_p(new_l);

                self.l = new_l;
            }
            // DCR L
            0x2D => {
                let (new_l, _) = self.l.overflowing_sub(1);

                // flags
                self.cc.z = get_z(new_l);
                self.cc.s = get_s(new_l);
                self.cc.p = get_p(new_l);

                self.l = new_l;
            }
            // INX SP
            0x33 => {
                let (new_sp, _) = self.sp.overflowing_add(1);

                self.sp = new_sp;
            }
            // INR M
            0x34 => {
                let (new_hl_mem, _) = self.memory[hl].overflowing_add(1);

                // flags
                self.cc.z = get_z(new_hl_mem);
                self.cc.s = get_s(new_hl_mem);
                self.cc.p = get_p(new_hl_mem);

                self.memory[hl] = new_hl_mem;
            }
            // DCR M
            0x35 => {
                let (new_hl_mem, _) = self.memory[hl].overflowing_sub(1);

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
                let (new_sp, _) = self.sp.overflowing_sub(1);

                self.sp = new_sp;
            }
            // INR A
            0x3C => {
                let (new_a, _) = self.a.overflowing_add(1);

                // flags
                self.cc.z = get_z(new_a);
                self.cc.s = get_s(new_a);
                self.cc.p = get_p(new_a);

                self.a = new_a;
            }
            // DCR A
            0x3D => {
                let (new_a, _) = self.a.overflowing_sub(1);

                // flags
                self.cc.z = get_z(new_a);
                self.cc.s = get_s(new_a);
                self.cc.p = get_p(new_a);

                self.a = new_a;
            }
            0x41 => self.b = self.c,                            // MOV B,C
            0x42 => self.b = self.d,                            // MOV B,D
            0x43 => self.b = self.e,                            // MOV B,E
            0x80 => self.a = self.add(self.a, self.b),          // ADD B
            0x81 => self.a = self.add(self.a, self.c),          // ADD C
            0x82 => self.a = self.add(self.a, self.d),          // ADD D
            0x83 => self.a = self.add(self.a, self.e),          // ADD E
            0x84 => self.a = self.add(self.a, self.h),          // ADD H
            0x85 => self.a = self.add(self.a, self.l),          // ADD L
            0x86 => self.a = self.add(self.a, self.memory[hl]), // ADD M (A = A + (HL))
            0x87 => self.a = self.add(self.a, self.a),          // ADD A (A = A + A)
            0x88 => self.a = self.adc(self.a, self.b),          // ADC B
            0x89 => self.a = self.adc(self.a, self.c),          // ADC C
            0x8A => self.a = self.adc(self.a, self.d),          // ADC D
            0x8B => self.a = self.adc(self.a, self.e),          // ADC E
            0x8C => self.a = self.adc(self.a, self.h),          // ADC H
            0x8D => self.a = self.adc(self.a, self.l),          // ADC L
            0x8E => self.a = self.adc(self.a, self.memory[hl]), // ADC M (A = A + (HL) + CY)
            0x8F => self.a = self.adc(self.a, self.a),          // ADC A (A = A + A + CY)
            0xC6 => self.a = self.add(self.a, self.memory[pc + 1]), // ADI D8 (rhs is an immediate value)
            0xCE => self.a = self.adc(self.a, self.memory[pc + 1]), // ACI D8 (rhs is an immediate value PLUS the carry flag value)
            0x90 => self.a = self.sub(self.a, self.b),              // SUB B
            0x91 => self.a = self.sub(self.a, self.c),              // SUB C
            0x92 => self.a = self.sub(self.a, self.d),              // SUB D
            0x93 => self.a = self.sub(self.a, self.e),              // SUB E
            0x94 => self.a = self.sub(self.a, self.h),              // SUB H
            0x95 => self.a = self.sub(self.a, self.l),              // SUB L
            0x96 => self.a = self.sub(self.a, self.memory[hl]),     // SUB M (A = A - (HL))
            0x97 => self.a = self.sub(self.a, self.a),              // SUB A (A = A - A)
            0x98 => self.a = self.sbb(self.a, self.b),              // SBB B
            0x99 => self.a = self.sbb(self.a, self.c),              // SBB C
            0x9A => self.a = self.sbb(self.a, self.d),              // SBB D
            0x9B => self.a = self.sbb(self.a, self.e),              // SBB E
            0x9C => self.a = self.sbb(self.a, self.h),              // SBB H
            0x9D => self.a = self.sbb(self.a, self.l),              // SBB L
            0x9E => self.a = self.sbb(self.a, self.memory[hl]),     // SBB M (A = A - (HL) - CY)
            0x9F => self.a = self.sbb(self.a, self.a),              // SBB A (A = A - A - CY)
            0xD6 => self.a = self.sub(self.a, self.memory[pc + 1]), // SUI D8 (rhs is an immediate value)
            0xDE => self.a = self.sbb(self.a, self.memory[pc + 1]), // SBI D8 (rhs is an immediate value MINUS the carry flag value)
            _ => panic!("Unknown opcode!"), // uncomment to determine the unimplemented opcodes
        }

        self.pc += 1;
    }
}
