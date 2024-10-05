use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "language.pest"]
pub struct RuleParser;
#[cfg(test)]
mod tests;
