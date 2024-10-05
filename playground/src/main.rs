// playground for informal testing
use rulelib::RuleParser;
use rulelib::Rule;
use pest::Parser;

fn main() {
    let text = "(def-var bad-ip \"192.0.0.1\")";
    let string = "\"192.0.0.1\"";
    let atom = "def-var";
    let s_expr = "(def-var)";
    let cons = "(def-var bad-ip (1 2))";
    let complex_rule = "(def-rule simple-rule (:target \"127.0.0.1\" :port \"80\") (if (and (exact? metadata-source bad-ip) (exact? metedata-dest   :target) DROP REDIRECT)))";
    let parse_result = RuleParser::parse(Rule::program, complex_rule);
    println!("{}", parse_result.unwrap());
}
