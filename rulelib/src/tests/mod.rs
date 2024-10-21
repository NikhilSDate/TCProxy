use crate::Rule;
use crate::RuleParser;
use pest::Parser;

#[test]
fn test_example_program() {
    let program = r#"
    (set-mode OPAQUE)

    (def-var bad-ip "192.0.1.2")

    (def-rule simple-rewrite
        (if (exact? metadata-source bad-ip)
            (REWRITE "^bar$" "baz")
            CONTINUE))

    (def-rule simple-rule
        (if (exact? metadata-source bad-ip)
            DROP
            (REDIRECT "127.0.0.1" 80)))
    "#;
    let parse_result = RuleParser::parse(Rule::program, program);
    assert!(parse_result.is_ok(), "parse failed on example program");
}

#[test]
fn test_bad_program() {
    // program has an unclosed paranthesis
    let bad_program = r#"
            (def-var bad-ip "192.0.1.2")

            (def-rule simple-rule (:target "127.0.0.1" :port   "80")
                (if (and (exact? metadata-source bad-ip) (exact? metedata-dest   :target)
                    DROP
                    REDIRECT)))

            (def-rule simple-rewrite
                (if (and (exact? metadata-source bad-ip) (matches? content "foo"))
                    (REWRITE "^bar$" "baz")
                    (simple-rule))
        "#;
    let parse_result = RuleParser::parse(Rule::program, bad_program);
    assert!(parse_result.is_err());
}

#[test]
fn test_boolean() {
    let program1 = "(def-var bool-var #t)";
    let program2 = "(def-var bool-var #f)";
    let expected = "[program(0, 21, [s_exp(0, 21, [list(0, 21, [s_exp(1, 8, [atom(1, 8, [ident(1, 8)])]), s_exp(9, 17, [atom(9, 17, [ident(9, 17)])]), s_exp(18, 20, [atom(18, 20, [bool(18, 20)])])])]), EOI(21, 21)])]";
    let parse1 = RuleParser::parse(Rule::program, program1);
    let parse2 = RuleParser::parse(Rule::program, program2);
    assert_eq!(parse1.unwrap().to_string(), expected);
    assert_eq!(parse2.unwrap().to_string(), expected);
}

#[test]
fn test_negative() {
    let negative = "-123";
    let parse_result = RuleParser::parse(Rule::number, negative);
    assert!(parse_result.is_ok());
}
