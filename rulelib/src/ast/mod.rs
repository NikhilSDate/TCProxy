use pest::iterators::Pair;
use crate::Rule;

#[derive(Debug, Copy, Clone)]
pub enum ProxyMode {
    OPAQUE,
    TRANSPARENT,
}

impl TryFrom<&str> for ProxyMode {
    type Error = AstParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "OPAQUE" => Ok(ProxyMode::OPAQUE),
            "TRANSPARENT" => Ok(ProxyMode::TRANSPARENT),
            _ => Err(AstParseError::ParseError(format!("Unknown proxy mode: {}", value)))
        }
    }
}

/// These are all meant to be "special forms," which have a different order of evaluation from typical terms;
/// for example, `(if a b c)` should only execute *either* the consequent or the alternative, depending on the truth value of `a`
/// See: https://www.cs.cmu.edu/Groups/AI/html/cltl/clm/node59.html
#[derive(Debug, Clone)]
pub enum SpecialForms {
    /// (if <predicate> <consequent> <alternative>)
    If {
        predicate: Box<AstNode>,
        consequent: Box<AstNode>,
        alternative: Box<AstNode>,
    },
    /// (def-var <name> <value>)
    DefVar {
        name: String,
        value: Box<AstNode>,
    },
    /// (def-rule <name> <body>)
    DefRule {
        name: String,
        // params: HashMap<String, String>,
        body: Box<AstNode>,
    },
    /// (set-mode OPAQUE) or (set-mode TRANSPARENT)
    SetMode {
        mode: ProxyMode
    },
}

// TODO: refactor
impl TryFrom<Pair<'_, Rule>> for SpecialForms {
    type Error = AstParseError;

    /// Tries to convert a parse tree node to a SpecialForm.
    /// Expects an `s_expr` as input
    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        // TODO: convert ParseError messages to enums
        match value.as_rule() {
            Rule::s_exp => Self::try_from(value.into_inner().next().expect("an `s_exp` is always either `list` or `ident`")),
            Rule::list => {
                let inner = value.into_inner().next();
                match inner {
                    // `nil`
                    None => Err(AstParseError::ParseError("expected `list_part`, found `nil`".to_string())),
                    // `list_part`
                    Some(inner) => Self::try_from(inner)
                }
            }
            Rule::list_part => {
                let inner: Vec<_> = value.into_inner().collect();
                match inner.first() {
                    None => unreachable!("`list_part` always contains at least one child"),
                    Some(expr) => {
                        match expr.as_str() {
                            "if" => {
                                // "if" + predicate + consequent + alternative
                                if inner.len() != 4 {
                                    Err(AstParseError::ParseError(format!("wrong arity for if; expected 3, received {}", inner.len() - 1)))
                                } else {
                                    // I think the clone here is necessary
                                    let predicate = Box::new(AstNode::try_from(inner[1].clone())?);
                                    let consequent = Box::new(AstNode::try_from(inner[2].clone())?);
                                    let alternative = Box::new(AstNode::try_from(inner[3].clone())?);

                                    Ok(Self::If { predicate, consequent, alternative })
                                }
                            }
                            "def-var" => {
                                // "def-var" + name + value
                                if inner.len() != 3 {
                                    Err(AstParseError::ParseError(format!("wrong arity for def-var; expected 2, received {}", inner.len() - 1)))
                                } else {
                                    // if (inner[1].as_rule != )

                                    // encore un fois (see comment in "if")
                                    todo!()
                                }
                            }
                            "def-rule" => { todo!() }
                            "set-mode" => {
                                // set-mode + OPAQUE/TRANSPARENT
                                if inner.len() != 2 {
                                    Err(AstParseError::ParseError(format!("wrong arity for set-mode; expected 1, received {}", inner.len() - 1)))
                                } else {
                                    let mode = ProxyMode::try_from(inner[1].as_str())?;
                                    Ok(Self::SetMode { mode })
                                }
                            }
                            _ => Err(AstParseError::ParseError(format!("expected a special form, received {}", expr.as_str())))
                        }
                    }
                }
            }
            rule => Err(AstParseError::ParseError(format!("expected `s_expr`, received {:?}", rule))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuleOutcome {
    /// Silently drop the inbound packet
    DROP,
    /// Respond with an ERR_CONNECTION_REFUSED
    REJECT,
    /// Forward the inbound packet to the specified redirect address
    REDIRECT {
        addr: String,
        port: u8,
    },
    /// Rewrite packet content via regex substitution
    REWRITE {
        pattern: String,
        replace_with: String,
    },
    // FIXME REMARK: added a CONTINUE outcome to make the piping semantics more obvious for chaining rules (fixme for visibility)
    /// Continue on to the next Rule
    CONTINUE,
}

#[derive(Debug, Clone)]
pub enum BuiltinOp {
    Func {
        name: String,
        arguments: Vec<Box<AstNode>>,
    },
    SpecialForm(SpecialForms),
    Outcome(RuleOutcome),
}

#[derive(Debug, Clone)]
pub enum List<T> {
    Nil,
    NonEmpty(Vec<T>),
}

#[derive(Debug, Clone)]
pub enum AstNode {
    // TODO: find a better name for this
    Keyword(BuiltinOp),
    Num(i64),
    Ident(String),
    String(String),
    Sexp(List<Box<AstNode>>),
}

impl TryFrom<Pair<'_, Rule>> for AstNode {
    type Error = AstParseError;

    fn try_from(value: Pair<'_, Rule>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum AstParseError {
    ParseError(String),
}

#[cfg(test)]
mod tests {
    // FIXME: leaky unit tests, but I don't want to manually write out parse trees...
    use pest::Parser;
    use pest_derive::Parser;
    use crate::{Rule, RuleParser};

    use crate::ast::AstParseError;

    mod proxy_mode {
        use super::*;
        use crate::ast::ProxyMode;

        #[test]
        fn try_from__works_on_valid_strings() {
            let proxy_mode = ProxyMode::try_from("OPAQUE");
            assert!(matches!(proxy_mode, Ok(ProxyMode::OPAQUE)));

            let proxy_mode = ProxyMode::try_from("TRANSPARENT");
            assert!(matches!(proxy_mode, Ok(ProxyMode::TRANSPARENT)));
        }

        #[test]
        fn try_from__fails_on_invalid_strings() {
            // bad capitalization?
            let proxy_mode = ProxyMode::try_from("OPaQUE");
            assert!(matches!(proxy_mode, Err(AstParseError::ParseError(_))));

            // random string?
            let proxy_mode = ProxyMode::try_from("oeau");
            assert!(matches!(proxy_mode, Err(AstParseError::ParseError(_))));
        }
    }

    mod special_forms {
        use super::*;
        use crate::ast::{ProxyMode, SpecialForms};

        #[test]
        fn try_from__fails_on_unexpected_parse_trees() {
            let parse_tree = RuleParser::parse(Rule::s_exp, "100").unwrap().next().unwrap();
            let ast = SpecialForms::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "hi").unwrap().next().unwrap();
            let ast = SpecialForms::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(bob was here)").unwrap().next().unwrap();
            let ast = SpecialForms::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(cow 100)").unwrap().next().unwrap();
            let ast = SpecialForms::try_from(parse_tree);
            assert!(ast.is_err());
        }


        mod set_mode {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_trees() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode TRANSPARENT)").unwrap().next().unwrap();

                let ast = SpecialForms::try_from(parse_tree).unwrap();
                assert!(matches!(ast, SpecialForms::SetMode { mode: ProxyMode::TRANSPARENT }));
            }

            #[test]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode OPAQUE hi)").unwrap().next().unwrap();

                let ast = SpecialForms::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode CANDY)").unwrap().next().unwrap();

                let ast = SpecialForms::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod r#if {
            use super::*;

            #[test]
            #[ignore]
            fn try_from__works_with_expected_parse_trees() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                todo!()
            }
        }
        mod def_var {
            use super::*;

            #[test]
            #[ignore]
            fn try_from__works_with_expected_parse_trees() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                todo!()
            }
        }

        mod def_rule {
            use super::*;

            #[test]
            #[ignore]
            fn try_from__works_with_expected_parse_trees() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                todo!()
            }

            #[test]
            #[ignore]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                todo!()
            }
        }
    }
}