use std::collections::HashMap;
use std::net::Ipv4Addr;

type Reg = usize;
type ObjKey = u32;
type Label = usize;

enum Instruction {
    SEQ(Reg, ObjKey, ObjKey), // set-if-equal
    AND(Reg, Reg, Reg),       // bitwise AND
    OR(Reg, Reg, Reg),        // bitwise OR
    NOT(Reg, Reg),       // bitwise NOT
    ITE(Reg, Label, Label),   // if-then-else
    DROP,
    REDIRECT(Label, Label), // redirect Address, Port,
    REJECT,
    REWRITE(Label, Label), // rewrite find_string replace_string
}

struct Program {
    instructions: Vec<Instruction>,
    data: HashMap<ObjKey, String>, // need to figure out how to support multiple object types later
}

const NUM_REGS: usize = 16;
struct VM {
    registers: [u32; NUM_REGS],
}

enum Action {
    DROP,
    REDIRECT(String, String), // address and port are just strings for now, should be specialized types later
    REWRITE(String, String), // action already taken, so no need to 
    REJECT,
}

enum Object {
    Ipv4Addr,
    u16,
    String
}



struct Packet {
    source: (Ipv4Addr, u16),
    dest: (Ipv4Addr, u16),
    content: Option<Vec<u8>>
}

// packet is just a string for now to test out find and replace, will need to define a packet struct eventually. 
impl VM {
    pub fn run_program(&mut self, program: &Program, packet: &Packet) -> Result<Action, &str> {
        self.init();
        let mut pc = 0; // program counter
        while pc < program.instructions.len() {
            let control_normal = true;
            match program.instructions[pc] {
                Instruction::SEQ(r0, key1, key2) => {
                    self.registers[r0] = (program.data[&key1] == program.data[&key2]) as u32;
                },
                Instruction::AND(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] & self.registers[r2];
                },
                Instruction::OR(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] | self.registers[r2];
                },
                Instruction::NOT(r0, r1) => {
                    self.registers[r0] = !self.registers[r1];
                },
                Instruction::ITE(r0, lab1, lab2) {
                    if self.registers[r0] != 0 {
                        pc = lab1;
                    } else {
                        pc = lab2;
                    }
                    control_normal = false;
                },
                Instruction::DROP => {
                    return Ok(Action::DROP);
                },
                Instruction::REDIRECT(address_label, port_label) => {
                    // handle invalid labels
                    return Ok(Action::REDIRECT(program.data[address_label], program.data[port_label]));
                },
                Instruction::REJECT => {
                    return Ok(Action::REJECT)
                },
                Instruction::REWRITE(find_label, replace_label) => {
                    return Ok(Action::REWRITE(program.data[find_label], program.data[replace_label]));
                }
                _ => {
                    panic!("Should never get here")
                }
            }
        }
        Err("Program ended without action")
    }

    // initialize all regs to 0
    pub fn init(&mut self) {
        // consider optimizing with mutable iterator
        for i in 0..self.registers.len() {
            self.registers[i] = 0;
        }
    }
}
