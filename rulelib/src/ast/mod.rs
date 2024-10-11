use crate::Rule;
use lazy_static::lazy_static;
use pest::iterators::Pair;
use std::collections::HashSet;
use std::net::IpAddr;

lazy_static! {
    static ref RESERVED_KEYWORDS: HashSet<&'static str> =
        HashSet::from(["def-var", "set-mode", "def-rule", "if"]);
}

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
            _ => Err(Self::Error::ParseError(format!(
                "Unknown proxy mode: {}",
                value
            ))),
        }
    }
}

/// These are all meant to be "special forms," which have a different order of evaluation from typical terms;
/// for example, `(if a b c)` should only execute *either* the consequent or the alternative, depending on the truth value of `a`
/// See: https://www.cs.cmu.edu/Groups/AI/html/cltl/clm/node59.html
#[derive(Debug, Clone)]
pub enum SpecialForm {
    /// (if <predicate> <consequent> <alternative>)
    If {
        predicate: Box<AstNode>,
        consequent: Box<AstNode>,
        alternative: Box<AstNode>,
    },
    /// (def-var <name> <value>)
    DefVar { name: String, value: Box<AstNode> },
    /// (def-rule <name> <body>)
    DefRule {
        name: String,
        // params: HashMap<String, String>,
        body: Box<AstNode>,
    },
    /// (set-mode OPAQUE) or (set-mode TRANSPARENT)
    SetMode { mode: ProxyMode },
}

impl SpecialForm {
    fn parse_if(inner: Vec<Pair<Rule>>) -> Result<Self, AstParseError> {
        // "if" + predicate + consequent + alternative
        if inner.len() != 4 {
            Err(AstParseError::ParseError(format!(
                "wrong arity for if; expected 3, received {}",
                inner.len() - 1
            )))
        } else {
            // FIXME: I believe the clones here are necessary
            let predicate = Box::new(AstNode::try_from(inner[1].clone())?);
            if !matches!(*predicate, AstNode::Ident(_) | AstNode::Sexp(_)) {
                return Err(AstParseError::ParseError(
                    "predicate must be an ident or Sexp".to_string(),
                ));
            }
            let consequent = Box::new(AstNode::try_from(inner[2].clone())?);
            let alternative = Box::new(AstNode::try_from(inner[3].clone())?);

            Ok(Self::If {
                predicate,
                consequent,
                alternative,
            })
        }
    }

    fn parse_def(
        special_form: &str,
        inner: Vec<Pair<Rule>>,
    ) -> Result<(String, Box<AstNode>), AstParseError> {
        // def-xxx + name + value
        if inner.len() != 3 {
            Err(AstParseError::ParseError(format!(
                "wrong arity for {}; expected 2, received {}",
                special_form,
                inner.len() - 1
            )))
        } else {
            let name = AstNode::try_from(inner[1].clone())?;
            if let AstNode::Ident(name) = name {
                let value = Box::new(AstNode::try_from(inner[2].clone())?);
                Ok((name, value))
            } else {
                Err(AstParseError::ParseError(format!(
                    "{} expected an `ident`, found {:?}",
                    special_form,
                    inner[1].as_rule()
                )))
            }
        }
    }

    fn parse_def_var(inner: Vec<Pair<Rule>>) -> Result<Self, AstParseError> {
        Self::parse_def("def-var", inner).map(|(name, value)| Self::DefVar { name, value })
    }

    fn parse_def_rule(inner: Vec<Pair<Rule>>) -> Result<Self, AstParseError> {
        Self::parse_def("def-rule", inner).map(|(name, body)| Self::DefRule { name, body })
    }

    fn parse_set_mode(inner: Vec<Pair<Rule>>) -> Result<Self, AstParseError> {
        // set-mode + OPAQUE/TRANSPARENT
        if inner.len() != 2 {
            Err(AstParseError::ParseError(format!(
                "wrong arity for set-mode; expected 1, received {}",
                inner.len() - 1
            )))
        } else {
            let mode = ProxyMode::try_from(inner[1].as_str())?;
            Ok(Self::SetMode { mode })
        }
    }
}

impl TryFrom<Pair<'_, Rule>> for SpecialForm {
    type Error = AstParseError;

    /// Tries to convert a parse tree node to a SpecialForm.
    /// Expects an `s_expr` as input
    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::s_exp => Self::try_from(
                value
                    .into_inner()
                    .next()
                    .expect("an `s_exp` is always either `list` or `atom`"),
            ),
            Rule::list => {
                let inner: Vec<_> = value.into_inner().collect();
                if inner.is_empty() {
                    Err(Self::Error::ParseError(
                        "expected `list_part`, found `nil`".to_string(),
                    ))
                } else {
                    match inner.first() {
                        None => unreachable!("`list_part` always contains at least one child"),
                        Some(expr) => match expr.as_str() {
                            "if" => Self::parse_if(inner),
                            "def-var" => Self::parse_def_var(inner),
                            "def-rule" => Self::parse_def_rule(inner),
                            "set-mode" => Self::parse_set_mode(inner),
                            _ => Err(Self::Error::ParseError(format!(
                                "expected a special form, received {}",
                                expr.as_str()
                            ))),
                        },
                    }
                }
            }
            rule => Err(Self::Error::ParseError(format!(
                "expected `s_expr`, received {:?}",
                rule
            ))),
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
    REDIRECT { addr: String, port: u8 },
    /// Rewrite packet content via regex substitution
    REWRITE {
        pattern: String,
        replace_with: String,
    },
    // FIXME REMARK: added a CONTINUE outcome to make the piping semantics more obvious for chaining rules (fixme for visibility)
    /// Continue on to the next Rule
    CONTINUE,
}

impl TryFrom<Pair<'_, Rule>> for RuleOutcome {
    type Error = AstParseError;

    /// Tries to convert a parse tree node to a RuleOutcome.
    /// Expects an `s_expr` as input
    fn try_from(value: Pair<'_, Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::s_exp => Self::try_from(
                value
                    .into_inner()
                    .next()
                    .expect("an `s_exp` is always either `list` or `atom`"),
            ),
            Rule::list => {
                let inner: Vec<_> = value.into_inner().collect();
                if inner.is_empty() {
                    Err(Self::Error::ParseError(
                        "expected `list_part`, found `nil`".to_string(),
                    ))
                } else {
                    match inner.first() {
                        None => unreachable!("`list_part` always contains at least one child"),
                        Some(expr) => {
                            match expr.as_str() {
                                "REDIRECT" => {
                                    // REDIRECT + ip addr + port
                                    if inner.len() != 3 {
                                        Err(Self::Error::ParseError(format!(
                                            "wrong arity for REDIRECT; expected 2, received {}",
                                            inner.len() - 1
                                        )))
                                    } else {
                                        // FIXME: sketchy way to get the string out; ideally, we recursively parse the AstNode and check if it's of variant `String`
                                        inner[1]
                                            .as_str()
                                            .trim_matches(|c| c == '"')
                                            .parse::<IpAddr>()
                                            .or(Err(Self::Error::ParseError(
                                                "bad address to REDIRECT".to_string(),
                                            )))
                                            .and_then(|addr| {
                                                // FIXME: similar remark here; ideally, we check if the AstNode is of variant `Num`
                                                inner[2]
                                                    .as_str()
                                                    .parse::<u8>()
                                                    .or(Err(Self::Error::ParseError(
                                                        "bad port to REDIRECT".to_string(),
                                                    )))
                                                    .and_then(|port| {
                                                        Ok(RuleOutcome::REDIRECT {
                                                            addr: addr.to_string(),
                                                            port,
                                                        })
                                                    })
                                            })
                                    }
                                }
                                "REWRITE" => {
                                    if inner.len() != 3 {
                                        Err(Self::Error::ParseError(format!(
                                            "wrong arity for REWRITE; expected 2, received {}",
                                            inner.len() - 1
                                        )))
                                    } else {
                                        let pattern = inner[1].as_str().trim_matches(|c| c == '"');
                                        let replace_with =
                                            inner[2].as_str().trim_matches(|c| c == '"');

                                        Ok(Self::REWRITE {
                                            pattern: pattern.to_string(),
                                            replace_with: replace_with.to_string(),
                                        })
                                    }
                                }
                                ident => Err(Self::Error::ParseError(format!(
                                    "expected one of `REDIRECT` or `REWRITE`, received {}",
                                    ident
                                ))),
                            }
                        }
                    }
                }
            }
            Rule::atom => Self::try_from(
                value
                    .into_inner()
                    .next()
                    .expect("an `atom` is always either `ident`, `number`, `string`"),
            ),
            Rule::ident => match value.as_str() {
                "DROP" => Ok(Self::DROP),
                "REJECT" => Ok(Self::REJECT),
                "CONTINUE" => Ok(Self::CONTINUE),
                ident => Err(Self::Error::ParseError(format!(
                    "expected one of `DROP` or `REJECT`, received {}",
                    ident
                ))),
            },
            rule => Err(Self::Error::ParseError(format!(
                "expected `s_expr`, received {:?}",
                rule
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Keyword {
    SpecialForm(SpecialForm),
    Outcome(RuleOutcome),
}

impl TryFrom<Pair<'_, Rule>> for Keyword {
    type Error = AstParseError;

    /// Tries to convert a parse tree node to a BuiltinOp.
    /// Expects an `s_expr` as input
    fn try_from(value: Pair<'_, Rule>) -> Result<Self, Self::Error> {
        if let Ok(form) = SpecialForm::try_from(value.clone()) {
            Ok(Self::SpecialForm(form))
        } else if let Ok(outcome) = RuleOutcome::try_from(value) {
            Ok(Self::Outcome(outcome))
        } else {
            Err(Self::Error::ParseError("not a builtin".to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub enum AstNode {
    // TODO: find a better name for this
    Keyword(Keyword),
    Num(i64),
    Ident(String),
    String(String),
    Sexp(Vec<AstNode>),
    Program(Vec<AstNode>),
}

impl TryFrom<Pair<'_, Rule>> for AstNode {
    type Error = AstParseError;

    /// Tries to convert a parse tree node to an AST.
    /// Expects an `s_expr` or a `program` as input
    fn try_from(value: Pair<'_, Rule>) -> Result<Self, Self::Error> {
        if let Ok(keyword) = Keyword::try_from(value.clone()) {
            Ok(AstNode::Keyword(keyword))
        } else {
            match value.as_rule() {
                Rule::program => {
                    let inner = value
                        .into_inner()
                        .filter(|v| !matches!(v.as_rule(), Rule::EOI))
                        .map(Self::try_from)
                        .collect::<Result<Vec<_>, _>>()?;

                    Ok(Self::Program(inner))
                }
                Rule::s_exp => Self::try_from(
                    value
                        .into_inner()
                        .next()
                        .expect("an `s_exp` is always either `list` or `atom`"),
                ),
                Rule::list => {
                    let inner = value
                        .into_inner()
                        .map(Self::try_from)
                        .collect::<Result<Vec<_>, _>>()?;

                    Ok(Self::Sexp(inner))
                }
                Rule::atom => Self::try_from(
                    value
                        .into_inner()
                        .next()
                        .expect("an `atom` is always either `ident`, `number`, `string`"),
                ),
                Rule::ident => {
                    let value = value.as_str();
                    if RESERVED_KEYWORDS.contains(value) {
                        Err(Self::Error::ParseError(format!(
                            "{} is a reserved keyword!",
                            value
                        )))
                    } else {
                        Ok(Self::Ident(value.to_string()))
                    }
                }
                Rule::string => Ok(Self::String(
                    value.as_str().trim_matches(|c| c == '"').to_string(),
                )),
                Rule::number => Ok(Self::Num(
                    value
                        .as_str()
                        .parse::<i64>()
                        .expect("`number` is guaranteed to be only ascii digits"),
                )),
                rule => Err(Self::Error::ParseError(format!(
                    "expected `s_expr`, received {:?}",
                    rule
                ))),
            }
        }
    }
}

// TODO: convert ParseError messages to enums
#[derive(Debug, Clone)]
pub enum AstParseError {
    ParseError(String),
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    // FIXME: leaky unit tests, but I don't want to manually write out parse trees...
    use crate::{Rule, RuleParser};
    use pest::Parser;

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
        use crate::ast::{AstNode, ProxyMode, SpecialForm};

        #[test]
        fn try_from__fails_on_unexpected_parse_trees() {
            let parse_tree = RuleParser::parse(Rule::s_exp, "100")
                .unwrap()
                .next()
                .unwrap();
            let ast = SpecialForm::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "hi")
                .unwrap()
                .next()
                .unwrap();
            let ast = SpecialForm::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(bob was here)")
                .unwrap()
                .next()
                .unwrap();
            let ast = SpecialForm::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(cow 100)")
                .unwrap()
                .next()
                .unwrap();
            let ast = SpecialForm::try_from(parse_tree);
            assert!(ast.is_err());
        }

        mod set_mode {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_trees() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode TRANSPARENT)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree).unwrap();
                assert!(matches!(
                    ast,
                    SpecialForm::SetMode {
                        mode: ProxyMode::TRANSPARENT
                    }
                ));
            }

            #[test]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode OPAQUE hi)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(set-mode CANDY)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod r#if {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_trees() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(if (placeholder) foo 85)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree).unwrap();
                assert!(matches!(
                    ast,
                    SpecialForm::If {
                        predicate,
                        consequent,
                        alternative
                    }
                    if matches!(*predicate, AstNode::Sexp(_)) && matches!(*consequent.clone(), AstNode::Ident(id) if id == "foo") && matches!(*alternative, AstNode::Num(n) if n == 85)
                ));
            }

            #[test]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(if)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(if 69 420 "foo")"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());

                let parse_tree =
                    RuleParser::parse(Rule::s_exp, r#"(if foo 420 (if "bar" baz cats))"#)
                        .unwrap()
                        .next()
                        .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod def_var {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_trees() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(def-var foo 420)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree).unwrap();
                assert!(matches!(ast, SpecialForm::DefVar {
                    name, value
                } if name == "foo" && matches!(*value, AstNode::Num(n) if n == 420)));
            }

            #[test]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(def-var foo 420 69)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(def-var 420 69)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());

                let parse_tree =
                    RuleParser::parse(Rule::s_exp, r#"(def-var foo (if "bar" baz cats))"#)
                        .unwrap()
                        .next()
                        .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod def_rule {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_trees() {
                let parse_tree = RuleParser::parse(
                    Rule::s_exp,
                    r#"(def-rule simple-rule (if (placeholder) 420 69))"#,
                )
                .unwrap()
                .next()
                .unwrap();

                let ast = SpecialForm::try_from(parse_tree).unwrap();
                assert!(
                    matches!(ast, SpecialForm::DefRule { name, body } if name == "simple-rule" && matches!(*body, AstNode::Keyword(_)))
                );
            }

            #[test]
            fn try_from__fails_on_parse_tree_with_wrong_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(def-rule foo bar baz)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_unexpected_argument() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "(def-rule 69 DROP)")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());

                let parse_tree =
                    RuleParser::parse(Rule::s_exp, r#"(def-rule foo (if "bar" baz cats))"#)
                        .unwrap()
                        .next()
                        .unwrap();

                let ast = SpecialForm::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }
    }

    mod rule_outcome {
        use super::*;
        use crate::ast::RuleOutcome;

        #[test]
        fn try_from__fails_on_unexpected_parse_trees() {
            let parse_tree = RuleParser::parse(Rule::s_exp, "100")
                .unwrap()
                .next()
                .unwrap();
            let ast = RuleOutcome::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "hi")
                .unwrap()
                .next()
                .unwrap();
            let ast = RuleOutcome::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(bob was here)")
                .unwrap()
                .next()
                .unwrap();
            let ast = RuleOutcome::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "(cow 100)")
                .unwrap()
                .next()
                .unwrap();
            let ast = RuleOutcome::try_from(parse_tree);
            assert!(ast.is_err());
        }

        mod drop {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_tree() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "DROP")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree).unwrap();
                assert!(matches!(ast, RuleOutcome::DROP));
            }

            #[test]
            fn try_from__fails_on_bad_capitalization() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "DrOP")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod reject {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_tree() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "REJECT")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree).unwrap();
                assert!(matches!(ast, RuleOutcome::REJECT));
            }

            #[test]
            fn try_from__fails_on_bad_capitalization() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "rEJECT")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod redirect {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_tree() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REDIRECT "127.0.0.1" 80)"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree).unwrap();
                assert!(
                    matches!(ast, RuleOutcome::REDIRECT {addr, port: 80} if addr == "127.0.0.1")
                );
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_invalid_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REDIRECT "127.0.0.1" 80 foo)"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_invalid_address() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REDIRECT "aaeuboa" 80)"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_invalid_port() {
                let parse_tree =
                    RuleParser::parse(Rule::s_exp, r#"(REDIRECT "127.0.0.1" 123213213112)"#)
                        .unwrap()
                        .next()
                        .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());

                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REDIRECT "127.0.0.1" "80")"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod rewrite {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_tree() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REWRITE "^bar$" "baz")"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree).unwrap();
                assert!(
                    matches!(ast, RuleOutcome::REWRITE {pattern, replace_with} if pattern == "^bar$" && replace_with == "baz")
                );
            }

            #[test]
            fn try_from__fails_on_well_formed_parse_tree_with_invalid_arity() {
                let parse_tree = RuleParser::parse(Rule::s_exp, r#"(REWRITE "^bar$" "baz" "foo")"#)
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }

        mod r#continue {
            use super::*;

            #[test]
            fn try_from__works_with_expected_parse_tree() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "CONTINUE")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree).unwrap();
                assert!(matches!(ast, RuleOutcome::CONTINUE));
            }

            #[test]
            fn try_from__fails_on_bad_capitalization() {
                let parse_tree = RuleParser::parse(Rule::s_exp, "CONTinue")
                    .unwrap()
                    .next()
                    .unwrap();

                let ast = RuleOutcome::try_from(parse_tree);
                assert!(ast.is_err());
            }
        }
    }

    mod ast_node {
        use super::*;
        use crate::ast::{AstNode, Keyword, ProxyMode, RuleOutcome, SpecialForm};

        #[test]
        fn try_from__works_correctly_on_atoms() {
            let parse_tree = RuleParser::parse(Rule::s_exp, r#"foo"#)
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree).unwrap();
            assert!(matches!(ast, AstNode::Ident(id) if id == "foo"));

            let parse_tree = RuleParser::parse(Rule::s_exp, r#"if"#)
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree);
            assert!(ast.is_err());

            let parse_tree = RuleParser::parse(Rule::s_exp, "69")
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree).unwrap();
            assert!(matches!(ast, AstNode::Num(69)));

            let parse_tree = RuleParser::parse(Rule::s_exp, r#""chicken nuggets""#)
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree).unwrap();
            assert!(matches!(ast, AstNode::String(s) if s == "chicken nuggets"));
        }

        #[test]
        fn try_from__works_on_lists() {
            let parse_tree = RuleParser::parse(Rule::s_exp, r#"(1 2 3 a "b" c)"#)
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree).unwrap();
            assert!(matches!(ast, AstNode::Sexp(_)));
            if let AstNode::Sexp(v) = ast {
                assert!(matches!(v[0], AstNode::Num(1)));
                assert!(matches!(v[1], AstNode::Num(2)));
                assert!(matches!(v[2], AstNode::Num(3)));
                assert!(matches!(v[3].clone(), AstNode::Ident(s) if s == "a"));
                assert!(matches!(v[4].clone(), AstNode::String(s) if s == "b"));
                assert!(matches!(v[5].clone(), AstNode::Ident(s) if s == "c"));
            }
        }

        #[test]
        fn try_from__works_on_sample_program() {
            let program = r#"
            (set-mode OPAQUE)

            (def-var bad-ip "192.0.1.2")

            (def-rule simple-rewrite
                (if (exact? :metadata-source bad-ip)
                    (REWRITE "^bar$" "baz")
                    CONTINUE))

            (def-rule simple-rule
                (if (exact? :metadata-source bad-ip)
                    DROP
                    (REDIRECT "127.0.0.1" 80)))
            "#;

            let parse_tree = RuleParser::parse(Rule::program, program)
                .unwrap()
                .next()
                .unwrap();
            let ast = AstNode::try_from(parse_tree).unwrap();

            if let AstNode::Program(stmts) = ast {
                assert_eq!(stmts.len(), 4);

                assert!(matches!(stmts[0].clone(), AstNode::Keyword(a)
                        if matches!(a.clone(), Keyword::SpecialForm(b)
                            if matches!(b.clone(), SpecialForm::SetMode {mode}
                                if matches!(mode, ProxyMode::OPAQUE)))));

                assert!(matches!(stmts[1].clone(), AstNode::Keyword(a)
                    if matches!(a.clone(), Keyword::SpecialForm(b)
                        if matches!(b.clone(), SpecialForm::DefVar {name, value}
                            if name == "bad-ip"
                            && matches!(*value.clone(), AstNode::String(s)
                                if s == "192.0.1.2")))));

                assert!(matches!(stmts[2].clone(), AstNode::Keyword(a)
                    if matches!(a.clone(), Keyword::SpecialForm(b)
                        if matches!(b.clone(), SpecialForm::DefRule {name, body}
                            if name == "simple-rewrite"
                            && matches!(*body.clone(), AstNode::Keyword(a)
                                if matches!(a.clone(), Keyword::SpecialForm(b)
                                    if matches!(b.clone(), SpecialForm::If {predicate, consequent, alternative}
                                        if matches!(*predicate.clone(), AstNode::Sexp(v)
                                            if matches!(v[0].clone(), AstNode::Ident(s)
                                                if s == "exact?")
                                            && matches!(v[1].clone(), AstNode::Ident(s)
                                                if s == ":metadata-source")
                                            && matches!(v[2].clone(), AstNode::Ident(s)
                                                if s == "bad-ip"))
                                        && matches!(*consequent.clone(), AstNode::Keyword(a)
                                            if matches!(a.clone(), Keyword::Outcome(o)
                                                if matches!(o.clone(), RuleOutcome::REWRITE {pattern, replace_with}
                                                    if pattern == "^bar$"
                                                    && replace_with == "baz")))
                                        && matches!(*alternative.clone(), AstNode::Keyword(a)
                                            if matches!(a.clone(), Keyword::Outcome(o)
                                                if matches!(o.clone(), RuleOutcome::CONTINUE))))))))));

                assert!(matches!(stmts[3].clone(), AstNode::Keyword(a)
                        if matches!(a.clone(), Keyword::SpecialForm(b)
                            if matches!(b.clone(), SpecialForm::DefRule {name, body}
                                if name == "simple-rule"
                                && matches!(*body.clone(), AstNode::Keyword(a)
                                    if matches!(a.clone(), Keyword::SpecialForm(b)
                                        if matches!(b.clone(), SpecialForm::If {predicate, consequent, alternative}
                                            if matches!(*predicate.clone(), AstNode::Sexp(v)
                                                if matches!(v[0].clone(), AstNode::Ident(s)
                                                    if s == "exact?")
                                                && matches!(v[1].clone(), AstNode::Ident(s)
                                                    if s == ":metadata-source")
                                                && matches!(v[2].clone(), AstNode::Ident(s)
                                                    if s == "bad-ip")) &&
                                            matches!(*consequent.clone(), AstNode::Keyword(a)
                                                if matches!(a.clone(), Keyword::Outcome(o)
                                                    if matches!(o.clone(), RuleOutcome::DROP))
                                                && matches!(*alternative.clone(), AstNode::Keyword(a)
                                                    if matches!(a.clone(), Keyword::Outcome(o)
                                                        if matches!(o.clone(), RuleOutcome::REDIRECT{addr, port}
                                                            if addr == "127.0.0.1" && port == 80)))))))))));
            } else {
                assert!(false);
            }
        }
    }
}
