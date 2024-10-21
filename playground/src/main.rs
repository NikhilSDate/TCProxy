// playground for informal testing
use pest::Parser;
use rulelib::Rule;
use rulelib::RuleParser;

fn main() {
    let program = "(def-var true-var #t)";
    let parse_result = RuleParser::parse(Rule::program, program);
    println!("{}", parse_result.unwrap());
    // let s_exp = parse_result.unwrap().next().unwrap().into_inner().next().unwrap();
    // let child3 = s_exp.into_inner().next().unwrap().into_inner().next().unwrap();
    // println!("{}", parse_result.unwrap());

}
