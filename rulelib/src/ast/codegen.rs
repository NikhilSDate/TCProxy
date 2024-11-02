use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::vm::{
    Instruction, Label, ObjKey, Object, Program, Reg, PACKET_CONTENT, PACKET_SOURCE_IP,
    PACKET_SOURCE_PORT,
};

const INVALID_PROGRAM: &'static str = "Precondition failed: Program is invalid";

#[derive(Debug, Default)]
struct AstCodeGenEnv {
    program: Program,
    names_to_keys: HashMap<String, ObjKey>,
    obj_key: ObjKey,
    curr_reg: Reg,
    curr_label: Label,
}

impl AstCodeGenEnv {
    fn insert_into_obj(&mut self, name: &str, obj: Object) -> ObjKey {
        let obj_key = self.obj_key;
        self.program.data.insert(obj_key, obj);
        self.names_to_keys.insert(name.into(), obj_key);
        self.obj_key += 1;

        obj_key
    }

    fn get_obj(&self, name: &str) -> Object {
        let key = self.names_to_keys.get(name).expect(INVALID_PROGRAM);
        self.program.data.get(key).expect(INVALID_PROGRAM).clone()
    }

    fn get_obj_key(&self, name: &str) -> ObjKey {
        *self.names_to_keys.get(name).expect(INVALID_PROGRAM)
    }

    fn add_instr(&mut self, instr: Instruction) -> Label {
        let curr_label = self.curr_label;
        self.program.instructions.push(instr);
        self.curr_label += 1;

        curr_label
    }

    fn update_instr(&mut self, label: Label, instr: Instruction) {
        std::mem::replace(
            self.program
                .instructions
                .get_mut(label)
                .expect("this instruction should already be in the instructions"),
            instr,
        );
    }
}

impl AstNode {
    /// Codegen from an `AstNode::Program`
    /// Assumes that program has already been validated via `AstNode::validate`
    pub fn codegen(&self) -> Program {
        match self {
            AstNode::Program(statements) => {
                let mut env = AstCodeGenEnv::default();

                // skip the proxy mode check since validation should catch that.
                for statement in statements.iter().skip(1) {
                    codegen_toplevel(&mut env, statement);
                }

                env.program
            }
            _ => unreachable!("AstNode::codegen should only be called on Programs!"),
        }
    }
}

fn codegen_toplevel(env: &mut AstCodeGenEnv, statement: &AstNode) {
    match statement {
        AstNode::Keyword(keyword) => match keyword {
            Keyword::SpecialForm(sf) => match sf {
                SpecialForm::DefVar { name, value } => {
                    codegen_var(env, name, value);
                }
                SpecialForm::DefRule { name, body } => {
                    codegen_rule(env, name, body);
                }
                _ => unreachable!("{}", INVALID_PROGRAM),
            },
            _ => unreachable!("{}", INVALID_PROGRAM),
        },
        _ => unreachable!("{}", INVALID_PROGRAM),
    }
}

// TODO: we don't acually need the name since we only allow for linear execution of rules.
fn codegen_rule(env: &mut AstCodeGenEnv, _name: &str, body: &AstNode) -> Label {
    match body {
        AstNode::Keyword(kw) => match kw {
            Keyword::SpecialForm(sf) => match sf {
                SpecialForm::If {
                    predicate,
                    consequent,
                    alternative,
                } => {
                    codegen_pred(env, predicate);
                    let curr_reg = env.curr_reg;
                    let ite = env.add_instr(Instruction::ITE(env.curr_reg, 0, 0));
                    let cons = codegen_rule(env, "", consequent);
                    let alt = codegen_rule(env, "", alternative);

                    env.update_instr(ite, Instruction::ITE(curr_reg, ite + 1, cons + 1));
                    alt + 1
                }
                _ => unreachable!("{}", INVALID_PROGRAM),
            },
            Keyword::Outcome(outcome) => codegen_outcome(env, &outcome),
        },
        _ => unreachable!("{}", INVALID_PROGRAM),
    }
}

fn codegen_pred(env: &mut AstCodeGenEnv, predicate: &AstNode) -> Label {
    match predicate {
        AstNode::Keyword(_) => todo!("haven't yet handled nested ifs"),
        AstNode::Bool(b) => {
            if *b {
                env.add_instr(Instruction::SEQ(0, 0, 0))
            } else {
                env.add_instr(Instruction::SEQ(0, 0, 0));
                env.add_instr(Instruction::NOT(0, 0))
            }
        }
        AstNode::Sexp(expr) => {
            let mut it = expr.iter();
            // FIXME: error message
            match it.next().unwrap() {
                AstNode::Ident(s) if s == "exact?" => codegen_exact(env, it.as_slice()),
                s => unimplemented!("{:?}", s),
            }
        }
        AstNode::Ident(_) => todo!("handle idents"),
        _ => unreachable!("{}", INVALID_PROGRAM),
    }
}

// NOTE: the arity should've been checked in valdiate
fn codegen_exact(env: &mut AstCodeGenEnv, statements: &[AstNode]) -> Label {
    let args1 = match &statements[0] {
        AstNode::Keyword(_) => todo!(),
        AstNode::Num(_) => todo!(),
        AstNode::Bool(_) => todo!(),
        AstNode::Ident(s) => match s.as_str() {
            ":packet-source-ip" => PACKET_SOURCE_IP,
            ":packet-source-port" => PACKET_SOURCE_PORT,
            ":packet-content" => PACKET_CONTENT,
            _ => env.get_obj_key(s)
        },
        AstNode::String(s) => env.insert_into_obj(
            &format!("{}", env.obj_key),
            Object::IP(s.parse().expect("Invalid IP")),
        ),
        AstNode::Sexp(_) => todo!(),
        AstNode::Program(_) => todo!(),
    };

    let args2 = match &statements[1] {
        AstNode::Keyword(_) => todo!(),
        AstNode::Num(_) => todo!(),
        AstNode::Bool(_) => todo!(),
        AstNode::Ident(s) => match s.as_str() {
            ":packet-source-ip" => PACKET_SOURCE_IP,
            ":packet-source-port" => PACKET_SOURCE_PORT,
            ":packet-content" => PACKET_CONTENT,
            _ => env.get_obj_key(s)
        },
        AstNode::String(s) => env.insert_into_obj(
            &format!("{}", env.obj_key),
            Object::IP(s.parse().expect("Invalid IP")),
        ),
        AstNode::Sexp(_) => todo!(),
        AstNode::Program(_) => todo!(),
    };

    env.add_instr(Instruction::SEQ(env.curr_reg, args1, args2))
}

fn codegen_var(env: &mut AstCodeGenEnv, name: &str, value: &AstNode) {
    // TODO: only works on atoms for now.
    match value {
        AstNode::Keyword(_) => todo!("We don't handle If yet"),
        AstNode::Num(n) => {
            // TODO: assume it's a port.
            env.insert_into_obj(name, Object::Port((*n).try_into().expect("invalid port")));
        }
        AstNode::Bool(_) => todo!("We don't handle Bools yet"),
        AstNode::Ident(ident) => {
            // FIXME: right now, this tkes the value associated with the ident and clones it;
            // that's really not necessary since we lack mutation---it would be better to just
            // store that item's obj_key
            let val = env.get_obj(ident);
            env.insert_into_obj(name, val);
        }
        AstNode::String(s) => {
            // TODO: for now, we only handle IPv4 addresses
            env.insert_into_obj(name, Object::IP(s.parse().expect("Invalid IP")));
        }
        AstNode::Sexp(_) => todo!("We don't handle Sexp's yet"),
        AstNode::Program(_) => unreachable!("{}", INVALID_PROGRAM),
    }
}

fn codegen_outcome(env: &mut AstCodeGenEnv, outcome: &RuleOutcome) -> Label {
    match outcome {
        RuleOutcome::DROP => env.add_instr(Instruction::DROP),
        RuleOutcome::REJECT => env.add_instr(Instruction::REJECT),
        // FIXME: RuleOutcome::REDIRECT should take a u16 for port.
        // TODO: implement lookup for symbols
        RuleOutcome::REDIRECT { addr, port } => {
            let addr = env.insert_into_obj(
                &(format!("{}", env.obj_key)),
                Object::IP(addr.parse().expect("Invalid IP")),
            );
            let port =
                env.insert_into_obj(&(format!("{}", env.obj_key)), Object::Port(*port as u16));
            env.add_instr(Instruction::REDIRECT(addr, port))
        }
        RuleOutcome::REWRITE {
            pattern,
            replace_with,
        } => {
            let pattern = env.insert_into_obj(
                &(format!("{}", env.obj_key)),
                Object::Data(Rc::new(pattern.as_bytes().to_owned())),
            );
            let replace_with = env.insert_into_obj(
                &(format!("{}", env.obj_key)),
                Object::Data(Rc::new(replace_with.as_bytes().to_owned())),
            );
            env.add_instr(Instruction::REWRITE(pattern, replace_with))
        }
        // TODO: to handle CONTINUEs, I'm thinking that we ought to keep track of the
        // current instruction, adding the jump to address only after we finish parsing
        // this current instruction.
        // Basically, we'll end up back-mutating the intructions list, perhaps via
        // `std::mem::replace`.
        RuleOutcome::CONTINUE => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use pest::Parser;

    use crate::RuleParser;

    use super::*;

    // TODO: tests
    #[test]
    fn test() {
        let program = r#"
            (set-mode OPAQUE)

            (def-var bad-ip "192.0.1.2")

            (def-rule simple-rule
                (if (exact? :packet-source-ip bad-ip)
                    DROP
                    (REDIRECT "127.0.0.1" 80)))
        "#;

        let parse_tree = RuleParser::parse(Rule::program, program)
            .unwrap()
            .next()
            .unwrap();
        let ast = AstNode::try_from(parse_tree).unwrap();

        let bytecode = AstNode::codegen(&ast);
        dbg!(&bytecode);
    }
}
