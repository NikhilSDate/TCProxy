use pest_derive::Parser;

// TODO: find a better name for this
mod ast;

#[derive(Parser)]
#[grammar = "language.pest"]
pub struct RuleParser;
#[cfg(test)]
mod tests;
pub mod vm;
