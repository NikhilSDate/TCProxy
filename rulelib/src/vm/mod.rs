use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

pub(crate) type Reg = usize;
pub(crate) type ObjKey = u32; // use positive numbers for HashMap keys, use negative numbers for packet fields
pub(crate) type Label = usize;

pub const PACKET_MASK: u32 = 0x80000000; // to access packet fields, set MSB of ObjKey to 1
pub const PACKET_SOURCE_IP: ObjKey = 0 | PACKET_MASK;
pub const PACKET_SOURCE_PORT: ObjKey = 1 | PACKET_MASK;
pub const PACKET_DEST_IP: ObjKey = 2 | PACKET_MASK;
pub const PACKET_DEST_PORT: ObjKey = 3 | PACKET_MASK;
pub const PACKET_CONTENT: ObjKey = 4 | PACKET_MASK;

#[derive(Debug, Clone)]
pub enum Instruction {
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

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub data: HashMap<ObjKey, Object>,
}

const NUM_REGS: usize = 16;
pub struct VM {
    registers: [u32; NUM_REGS],
}

#[derive(PartialEq, Debug)]
pub enum Action {
    DROP,
    REDIRECT(Object, Object),
    REWRITE(Object, Object),
    REJECT,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Object {
    IP(Ipv4Addr),
    Port(u16),
    Data(Arc<Vec<u8>>), // TODO: make this a lifetime
}

pub struct Packet {
    pub source: (Ipv4Addr, u16),
    pub dest: (Ipv4Addr, u16),
    pub content: Arc<Vec<u8>>,
}

impl VM {
    pub fn new() -> Self {
        let regs = [0; NUM_REGS];
        Self { registers: regs }
    }

    /// Precondition: program is a valid Program (has valid register numbers and labels)
    pub fn run_program(&mut self, program: &Program, packet: &Packet) -> Result<Action, &str> {
        let mut pc = 0; // program counter
        while pc < program.instructions.len() {
            let mut control_normal = true;
            match program.instructions[pc] {
                Instruction::SEQ(r0, key1, key2) => {
                    self.registers[r0] = (self.get_object(key1, program, packet)
                        == self.get_object(key2, program, packet))
                        as u32;
                }
                Instruction::AND(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] & self.registers[r2];
                }
                Instruction::OR(r0, r1, r2) => {
                    self.registers[r0] = self.registers[r1] | self.registers[r2];
                }
                Instruction::NOT(r0, r1) => {
                    self.registers[r0] = !self.registers[r1];
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
    pub fn get_object(
        &self,
        key: ObjKey,
        program: &Program,
        packet: &Packet,
    ) -> Result<Object, &str> {
        if key & PACKET_MASK == 0 {
            Ok(program.data[&key].clone())
        } else {
            match key {
                PACKET_SOURCE_IP => Ok(Object::IP(packet.source.0)),
                PACKET_SOURCE_PORT => Ok(Object::Port(packet.source.1)),
                PACKET_DEST_IP => Ok(Object::IP(packet.source.0)),
                PACKET_DEST_PORT => Ok(Object::Port(packet.source.1)),
                PACKET_CONTENT => Ok(Object::Data(packet.content.clone())),
                _ => Err("Invalid key"),
            }
        }
    }

    // reset all regs to 0
    pub fn reset(&mut self) {
        // consider optimizing with mutable iterator
        self.registers.iter_mut().for_each(|x| *x = 0);
    }
}

#[cfg(test)]
mod tests {
    use super::Instruction::*;
    use super::*;

    use crate::ast::AstNode;
    use crate::parser::RuleParser;
    use pest::Parser;
    use crate::parser::Rule;

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
            content: Arc::new(vec![]),
        };
        vm.run_program(&program, &packet);
        assert_eq!(vm.registers[0], 1);
        assert_eq!(vm.registers[1], 0);
    }

    #[test]
    pub fn test_ip_equals() {
        let insns = vec![
            SEQ(0, 0, PACKET_SOURCE_IP),
            ITE(0, 2, 3),
            DROP,
            REDIRECT(1, 2)
        ];
        let mut data = HashMap::new();
        data.insert(0, Object::IP(Ipv4Addr::new(123, 123, 123, 123)));
        data.insert(1, Object::IP(Ipv4Addr::new(128, 128, 128, 128)));
        data.insert(2, Object::Port(443));
        let program = Program {
            instructions: insns,
            data: data,
        };
        let mut vm = VM::new();
        let packet = Packet {
            source: (Ipv4Addr::new(123, 123, 123, 123), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Arc::new(vec![]),
        };
        let result = vm.run_program(&program, &packet);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Action::DROP);
    }

    #[test]
    pub fn test_vm_data() {
        let insns: Vec<Instruction> = vec![Instruction::SEQ(5, 0, 1)];
        let mut data = HashMap::new();
        data.insert(0, Object::Data(Arc::new(vec![1, 2, 3])));
        data.insert(1, Object::Data(Arc::new(vec![1, 2, 3])));
        let program = Program {
            instructions: insns,
            data: data,
        };
        let mut vm = VM::new();
        let packet = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Arc::new(vec![]),
        };
        vm.run_program(&program, &packet);
        assert_eq!(vm.registers[5], 1);
        assert_eq!(vm.registers[1], 0);
    }

    #[test]
    pub fn test_logical() {
        let mut data = HashMap::new();
        data.insert(0, Object::Data(Arc::new(vec![1, 4, 8])));
        data.insert(1, Object::Data(Arc::new(vec![1, 4, 8])));
        data.insert(2, Object::Port(443));
        let insns = vec![
            Instruction::SEQ(0, 0, 1),
            Instruction::SEQ(1, 0, 2),
            Instruction::OR(2, 0, 1),
            Instruction::AND(3, 0, 1),
            Instruction::ITE(2, 5, 6),
            Instruction::NOT(5, 5),
            Instruction::DROP,
            Instruction::REJECT,
        ];
        let program = Program {
            instructions: insns,
            data: data,
        };
        let packet = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Arc::new(vec![]),
        };
        let mut vm = VM::new();
        let result = vm.run_program(&program, &packet);
        assert!(result.is_ok());
        assert!(result.unwrap() == Action::DROP);
        assert_eq!(vm.registers[2], 1);
        assert_eq!(vm.registers[3], 0);
        assert_eq!(vm.registers[5], !0);
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

    #[test]
    pub fn test_redirect_rewrite() {
        let mut vm = VM::new();
        let mut data = HashMap::new();

        let find = Object::Data(Arc::new(vec![0x41]));
        let replace = Object::Data(Arc::new(vec![0x61]));

        let redirect_ip = Object::IP(Ipv4Addr::new(123, 123, 123, 123));
        let redirect_port = Object::Port(442);

        data.insert(0, Object::Data(Arc::new(vec![0x41, 0x41, 0x41])));
        data.insert(1, find.clone());
        data.insert(2, replace.clone());
        data.insert(3, redirect_ip.clone());
        data.insert(4, redirect_port.clone());

        let packet1 = Packet {
            source: (Ipv4Addr::new(0, 0, 0, 0), 16),
            dest: (Ipv4Addr::new(0, 0, 0, 0), 16),
            content: Arc::new(vec![0x41, 0x41, 0x41]),
        };

        let packet2 = Packet {
            content: Arc::new(vec![0x42, 0x42, 0x42]),
            ..packet1
        };
        let insns = vec![
            SEQ(0, PACKET_CONTENT, 0),
            ITE(0, 2, 3),
            REWRITE(1, 2),
            REDIRECT(3, 4),
        ];

        let program = Program {
            data: data,
            instructions: insns,
        };

        // test with packet that goes to if
        let result1 = vm.run_program(&program, &packet1);
        assert!(result1.is_ok());
        let action1 = result1.unwrap();
        assert_eq!(action1, Action::REWRITE(find, replace));

        vm.reset();

        // test with packet that goes to else
        let result2 = vm.run_program(&program, &packet2);
        assert!(result2.is_ok());
        let action2 = result2.unwrap();
        assert_eq!(action2, Action::REDIRECT(redirect_ip, redirect_port));
    }

    fn test_program_helper<'a>(program: &'a str, vm: &'a mut VM, packet: &Packet) -> Result<Action, &'a str> {
        let parse_tree = RuleParser::parse(Rule::program, program)
            .unwrap()
            .next()
            .unwrap();
        let ast = AstNode::try_from(parse_tree).unwrap();
        let bytecode = AstNode::codegen(&ast);
        vm.run_program(&bytecode, packet)
    }

    #[test]
    pub fn test_simple_program() {
        let program = r#"
        (set-mode OPAQUE)

        (def-var bad-ip "192.0.1.2")

        (def-rule simple-rule
            (if (exact? :packet-source-ip bad-ip)
                DROP
                (REDIRECT "127.0.0.1" 80)))
        "#;
        let bad_ip = Ipv4Addr::new(192, 0, 1, 2);
        let good_ip = Ipv4Addr::new(192, 168, 0, 1);
        let dest_ip = Ipv4Addr::new(192, 168, 1, 1);
        let content: Vec<u8> = vec![];
        let bad_packet = Packet {
            source: (bad_ip, 80),
            dest: (dest_ip, 80),
            content: Arc::new(content.clone())
        };
        let good_packet = Packet {
            source: (good_ip, 80),
            dest: (dest_ip, 80),
            content: Arc::new(content.clone())
        };
        let mut vm = VM::new();
        let bad_action = test_program_helper(program, &mut vm, &bad_packet).unwrap();
        let bad_action_target = Action::DROP;
        assert_eq!(bad_action, bad_action_target);
        let good_action = test_program_helper(program, &mut vm, &good_packet).unwrap();
        let good_action_target = Action::REDIRECT(Object::IP(Ipv4Addr::new(127, 0, 0, 1)), Object::Port(80));
        assert_eq!(good_action, good_action_target);
    }
}
