// playground for informal testing
use pest::Parser;
use rulelib::Rule;
use rulelib::RuleParser;

fn main() {
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
    println!("{}", parse_result.unwrap());
}
