const MEMORY_SIZE: usize = 65_536; // 2 ^ 16, 16-bit addresses

// TODO: default values? check emulator101.com code for more info on `z:1`, etc.
struct ConditionCodes {
    z: u8,
    s: u8,
    p: u8,
    cy: u8,
    ac: u8,
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

fn parity(num: u8) -> u8 {
    if num % 2 == 0 {
        1
    } else {
        0
    }
}

impl State8080 {
    // for ADD and ADI instructions
    fn add(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_add(rhs);

        // Z flag
        if ans == 0 {
            self.cc.z = 1;
        } else {
            self.cc.z = 0;
        }

        // S flag
        if ans >> 7 == 1 {
            self.cc.s = 1;
        } else {
            self.cc.s = 0;
        }

        // CY flag
        if has_overflowed {
            self.cc.cy = 1;
        } else {
            self.cc.cy = 0;
        }

        // P flag
        self.cc.p = parity(ans & 0xFF);

        ans
    }

    // for ADC (and ACI) instructions
    fn adc(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_add(rhs);

        // Z flag
        if ans == 0 {
            self.cc.z = 1;
        } else {
            self.cc.z = 0;
        }

        // S flag
        if ans >> 7 == 1 {
            self.cc.s = 1;
        } else {
            self.cc.s = 0;
        }

        // CY flag
        if has_overflowed {
            self.cc.cy = 1;
        } else {
            self.cc.cy = 0;
        }

        // P flag
        self.cc.p = parity(ans & 0xFF);

        ans + self.cc.cy // TODO: maybe this will overflow?
    }

    // for SUB and SUI instructions
    fn sub(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_sub(rhs);

        // Z flag
        if ans == 0 {
            self.cc.z = 1;
        } else {
            self.cc.z = 0;
        }

        // S flag
        if ans >> 7 == 1 {
            self.cc.s = 1;
        } else {
            self.cc.s = 0;
        }

        // CY flag
        if has_overflowed {
            self.cc.cy = 1;
        } else {
            self.cc.cy = 0;
        }

        // P flag
        self.cc.p = parity(ans & 0xFF);

        ans
    }

    // for SBB instructions
    fn sbb(&mut self, lhs: u8, rhs: u8) -> u8 {
        let (ans, has_overflowed) = lhs.overflowing_sub(rhs);

        // Z flag
        if ans == 0 {
            self.cc.z = 1;
        } else {
            self.cc.z = 0;
        }

        // S flag
        if ans >> 7 == 1 {
            self.cc.s = 1;
        } else {
            self.cc.s = 0;
        }

        // CY flag
        if has_overflowed {
            self.cc.cy = 1;
        } else {
            self.cc.cy = 0;
        }

        // P flag
        self.cc.p = parity(ans & 0xFF);

        ans - self.cc.cy // TODO: maybe this will overflow?
    }

    fn emulate_cycle(&mut self) {
        // fetch opcode
        let pc = self.pc as usize;
        let opcode = self.memory[pc];
        let hl = ((self.h as usize) << 8) | self.l as usize; // TODO: research more on `.into()`

        match opcode {
            // NOP
            0x00 | 0x08 | 0x10 | 0x18 | 0x28 | 0x38 | 0xCB | 0xD9 | 0xDD | 0xED | 0xFD => (),
            // LXI B,D16
            0x01 => {
                self.c = self.memory[pc + 1];
                self.b = self.memory[pc + 2];
                self.pc += 2;
            }
            // STAX B
            0x02 => {
                // TODO:
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
