// playground for informal testing
use pest::Parser;
use rulelib::Rule;
use rulelib::RuleParser;

fn main() {
    let program = "(def-var true-var #t)";
    let parse_result = RuleParser::parse(Rule::program, program);
    let expected_parse = "[program(0, 21, [s_exp(0, 21, [list(0, 21, [list_part(1, 20, [s_exp(1, 8, [atom(1, 8, [ident(1, 8)])]), list_part(9, 20, [s_exp(9, 17, [atom(9, 17, [ident(9, 17)])]), list_part(18, 20, [s_exp(18, 20, [atom(18, 20, [bool(18, 20)])])])])])])]), EOI(21, 21)])]";
    // let s_exp = parse_result.unwrap().next().unwrap().into_inner().next().unwrap();
    // let child3 = s_exp.into_inner().next().unwrap().into_inner().next().unwrap();
    // println!("{}", parse_result.unwrap());
    assert_eq!(parse_result.unwrap().to_string(), expected_parse);
}
