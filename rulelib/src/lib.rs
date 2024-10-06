
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CSVParser;
#[cfg(test)]
mod tests {
    // Included temporarily to test GitHub Actions workflow
    fn sanity_check() {
        assert_eq!(1+1, 2);
    }
}
