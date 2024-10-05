pub enum ProxyMode {
    OPAQUE,
    TRANSPARENT,
}

/// These are all meant to be "special forms," which have a different order of evaluation from typical terms;
/// for example, `(if a b c)` should only execute *either* the consequent or the alternative, depending on the truth value of `a`
/// See: https://www.cs.cmu.edu/Groups/AI/html/cltl/clm/node59.html
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

pub enum BuiltinOp {
    Func {
        name: String,
        arguments: Vec<Box<AstNode>>,
    },
    SpecialForm(SpecialForms),
    Outcome(RuleOutcome),
}

pub enum AstNode {
    // TODO: find a better name for this
    Keyword(BuiltinOp),
    Num(i64),
    Ident(String),
    String(String),
    Sexp(Vec<Box<AstNode>>)
}