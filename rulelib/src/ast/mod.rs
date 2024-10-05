use pest::iterators::Pair;
use crate::Rule;

#[derive(Debug)]
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
#[derive(Debug)]
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


#[derive(Debug)]
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

#[derive(Debug)]
pub enum BuiltinOp {
    Func {
        name: String,
        arguments: Vec<Box<AstNode>>,
    },
    SpecialForm(SpecialForms),
    Outcome(RuleOutcome),
}

#[derive(Debug)]
pub enum List<T> {
    Nil,
    NonEmpty(Vec<T>),
}

#[derive(Debug)]
pub enum AstNode {
    // TODO: find a better name for this
    Keyword(BuiltinOp),
    Num(i64),
    Ident(String),
    String(String),
    Sexp(List<Box<AstNode>>),
}

#[derive(Debug)]
pub enum AstParseError {
    ParseError(String),
}

#[cfg(test)]
mod tests {
    mod proxy_mode {
        use crate::ast::{AstParseError, ProxyMode};

        #[test]
        fn try_from_works_on_valid_strings() {
            let proxy_mode = ProxyMode::try_from("OPAQUE");
            assert!(matches!(proxy_mode, Ok(ProxyMode::OPAQUE)));

            let proxy_mode = ProxyMode::try_from("TRANSPARENT");
            assert!(matches!(proxy_mode, Ok(ProxyMode::TRANSPARENT)));
        }

        #[test]
        fn try_from_fails_on_invalid_strings() {
            // bad capitalization?
            let proxy_mode = ProxyMode::try_from("OPaQUE");
            assert!(matches!(proxy_mode, Err(AstParseError::ParseError(_))));

            // random string?
            let proxy_mode = ProxyMode::try_from("oeau");
            assert!(matches!(proxy_mode, Err(AstParseError::ParseError(_))));
        }
    }
}