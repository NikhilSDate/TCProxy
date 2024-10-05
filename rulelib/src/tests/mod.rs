use crate::RuleParser;
use pest::Parser;
use crate::Rule;

#[test]
fn test_example_program() {
    let program = r#"
            (def-var bad-ip "192.0.1.2")

            (def-rule simple-rule (:target "127.0.0.1" :port   "80")
                (if (and (exact? metadata-source bad-ip) (exact? metedata-dest   :target)
                    DROP
                    REDIRECT)))

            (def-rule simple-rewrite
                (if (and (exact? metadata-source bad-ip) (matches? content "foo"))
                    (REWRITE "^bar$" "baz")
                    (simple-rule)))
        "#;
    let parse_result = RuleParser::parse(Rule::program, program);
    assert!(parse_result.is_ok(), "parse failed on example program");
}
