use anyhow::{anyhow, Result};
use regex::Regex;
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader},
    ops::Range,
};

pub type GroupRanges = Vec<(String, Range<usize>)>;

pub fn read_groups_file(file_path: &str) -> Result<GroupRanges> {
    parse_groups_file_txt(&fs::read_to_string(file_path)?)
}

/// Parse a file contents defining group ranges with format:
/// - Index range in flexible format before first space
/// - Everything after first space is the group name str
/// ```txt
/// 0..1000 entityA lighthouse-geth-0
/// 1000..2000 entityB lodestar-nethermind-0
/// ```
fn parse_groups_file_txt(content: &str) -> Result<GroupRanges> {
    let mut result = Vec::new();

    for line in content.lines() {
        if let Some(space_index) = line.find(' ') {
            let (range_str, name) = line.split_at(space_index);
            let name = name.trim().to_string();
            let range = parse_range(range_str)?;
            result.push((name, range));
        }
    }

    Ok(result)
}

/// Parses a string representing a range with format:
/// "0-10", "0..10", "[0..10]", "[0-10]", "(0..10)", "[0-10)",
fn parse_range(input: &str) -> Result<Range<usize>> {
    let re = Regex::new(r"(\d+)[-.]+(\d+)")?;
    let captures = re
        .captures(input)
        .ok_or_else(|| anyhow!("Invalid input format"))?;
    let start: usize = captures[1].parse()?;
    let end: usize = captures[2].parse()?;
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_range_test() {
        let inputs = ["0-10", "0..10", "[0..10]", "[0-10]", "(0..10)", "[0-10)"];

        for input in inputs {
            assert_eq!(parse_range(&input).unwrap(), 0..10);
        }
    }

    #[test]
    fn read_file() {
        assert!(true)
    }
}
