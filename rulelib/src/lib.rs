use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "language.pest"]
pub struct RuleParser;
#[cfg(test)]
pub mod ast;
pub mod parser;
pub mod vm;
