use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::rc::Rc;

type Reg = usize;
type ObjKey = u32; // use positive numbers for HashMap keys, use negative numbers for packet fields
type Label = usize;


const PacketMask: u32 = 0x80000000; // to access packet fields, set MSB of ObjKey to 1
const PacketSourceIP: ObjKey = 0;
const PacketSourcePort: ObjKey = 1;
const PacketDestIP: ObjKey = 2;
const PacketDestPort: ObjKey = 3;
const PacketContent: ObjKey = 4;

enum Instruction {
    SEQ(Reg, ObjKey, ObjKey), // set-if-equal
    AND(Reg, Reg, Reg),       // bitwise AND
    OR(Reg, Reg, Reg),        // bitwise OR
    NOT(Reg, Reg),            // bitwise NOT
    ITE(Reg, Label, Label),   // if-then-else
    DROP,
    REDIRECT(ObjKey, ObjKey), // redirect Address, Port,
    REJECT,
    REWRITE(ObjKey, ObjKey), // rewrite find_string replace_string
}

struct Program {
    instructions: Vec<Instruction>,
    data: HashMap<ObjKey, Object>, // need to figure out how to support multiple object types later
}

const NUM_REGS: usize = 16;
struct VM {
    registers: [u32; NUM_REGS],
}

#[derive(PartialEq)]
enum Action {
    DROP,
    REDIRECT(Object, Object), // address and port are just strings for now, should be specialized types later
    REWRITE(Object, Object),  // action already taken, so no need to
    REJECT,
}

#[derive(PartialEq, Clone)]
enum Object {
    IP(Ipv4Addr),
    Port(u16),
    Data(Rc<Vec<u8>>), // TODO: make this a lifetime
}

struct Packet {
    source: (Ipv4Addr, u16),
    dest: (Ipv4Addr, u16),
    content: Rc::<Vec<u8>>,
}

impl VM {
    pub fn new() -> Self {
        let regs = [0; NUM_REGS];
        Self { registers: regs }
    }

    pub fn run_program(&mut self, program: &Program, packet: &Packet) -> Result<Action, &str> {
        let mut pc = 0; // program counter
        while pc < program.instructions.len() {
            let mut control_normal = true;
            match program.instructions[pc] {
                Instruction::SEQ(r0, key1, key2) => {
                    self.registers[r0] = (self.get_object(key1, program, packet) == self.get_object(key2, program, packet)) as u32;
                }
                Instruction::AND(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] & self.registers[r2];
                }
                Instruction::OR(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] | self.registers[r2];
                }
                Instruction::NOT(r0, r1) => {
                    self.registers[r0] = (!self.registers[r1]) & 1;
                }
                Instruction::ITE(r0, lab1, lab2) => {
                    if self.registers[r0] != 0 {
                        pc = lab1;
                    } else {
                        pc = lab2;
                    }
                    control_normal = false;
                }
                Instruction::DROP => {
                    return Ok(Action::DROP);
                }
                Instruction::REDIRECT(address_label, port_label) => {
                    // handle invalid labels
                    return Ok(Action::REDIRECT(
                        program.data[&address_label].clone(),
                        program.data[&port_label].clone(),
                    ));
                }
                Instruction::REJECT => return Ok(Action::REJECT),
                Instruction::REWRITE(find_label, replace_label) => {
                    return Ok(Action::REWRITE(
                        program.data[&find_label].clone(),
                        program.data[&replace_label].clone(),
                    ));
                }
            }
            if control_normal {
                pc += 1;
            }
        }
        Err("Program ended without action")
    }

    // this is the "memory controller"
    pub fn get_object(&self, key: ObjKey, program: &Program, packet: &Packet) -> Result<Object, &str> {
        if key & PacketMask == 0 {
            Ok(program.data[&key].clone())
        } else {
            match key & !PacketMask {
                PacketSourceIP => Ok(Object::IP(packet.source.0)),
                PacketSourcePort => Ok(Object::Port(packet.source.1)),
                PacketDestIP => Ok(Object::IP(packet.source.0)),
                PacketDestPort => Ok(Object::Port(packet.source.1)),
                PacketContent => Ok(Object::Data(packet.content.clone())),
                _ => Err("Invalid key")
            }
        }
    }

    // reset all regs to 0
    pub fn reset(&mut self) {
        // consider optimizing with mutable iterator
        for i in 0..self.registers.len() {
            self.registers[i] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_vm_seq() {
        let insns = vec![Instruction::SEQ(0, 0, 1)];
        let mut data = HashMap::new();
        data.insert(0, Object::Port(10));
        data.insert(1, Object::Port(10));
        let program = Program {
            instructions: insns,
            data: data,
        };
        let mut vm = VM::new();
        let packet = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Rc::new(vec![]),
        };
        vm.run_program(&program, &packet);
        assert_eq!(vm.registers[0], 1);
        assert_eq!(vm.registers[1], 0);
    }

    #[test]
    pub fn test_vm_data() {
        let insns: Vec<Instruction> = vec![Instruction::SEQ(5, 0, 1)];
        let mut data = HashMap::new();
        data.insert(0, Object::Data(Rc::new(vec![1, 2, 3])));
        data.insert(1, Object::Data(Rc::new(vec![1, 2, 3])));
        let program = Program {
            instructions: insns,
            data: data,
        };
        let mut vm = VM::new();
        let packet = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Rc::new(vec![]),
        };
        vm.run_program(&program, &packet);
        assert_eq!(vm.registers[5], 1);
        assert_eq!(vm.registers[1], 0);
    }

    #[test]
    pub fn test_logical() {
        let mut data = HashMap::new();
        data.insert(0, Object::Data(Rc::new(vec![1, 4, 8])));
        data.insert(1, Object::Data(Rc::new(vec![1, 4, 8])));
        data.insert(2, Object::Port(443));
        let insns = vec![Instruction::SEQ(0, 0, 1), Instruction::SEQ(1, 0, 2), Instruction::OR(2, 0, 1), Instruction::AND(3, 0, 1), Instruction::ITE(2, 5, 6), Instruction::NOT(5, 5), Instruction::DROP, Instruction::REJECT];
        let program = Program {
            instructions: insns,
            data: data
        };
        let packet = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Rc::new(vec![]),
        };
        let mut vm = VM::new();
        let result = vm.run_program(&program, &packet);
        assert!(result.is_ok());
        assert!(result.unwrap() == Action::DROP);
        assert_eq!(vm.registers[2], 1);
        assert_eq!(vm.registers[3], 0);
        assert_eq!(vm.registers[5], 1);
    }

    #[test]
    pub fn test_reset() {
        let mut vm = VM::new();
        vm.registers[0] = 1;
        vm.registers[3] = 1;
        vm.reset();
        assert_eq!(vm.registers[0], 0);
        assert_eq!(vm.registers[3], 0);
    }
}

