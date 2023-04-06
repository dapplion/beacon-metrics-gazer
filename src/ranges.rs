use anyhow::{anyhow, Result};
use regex::Regex;
use std::{collections::HashMap, ops::Range};

pub type IndexRanges = Vec<(String, Range<usize>)>;
type IndexRangesJson = HashMap<String, String>;

/// Parse group file contents flexibly, either as JSON first or then TXT
pub fn parse_ranges(input: &str) -> Result<IndexRanges> {
    if let Ok(groups) = parse_ranges_as_json(input) {
        return Ok(groups);
    }

    parse_ranges_as_txt(input)
}

/// Parse a file contents defining group ranges with format:
/// - Index range in flexible format before first space
/// - Everything after first space is the group name str
/// ```txt
/// 0..1000 entityA lighthouse-geth-0
/// 1000..2000 entityB lodestar-nethermind-0
/// ```
fn parse_ranges_as_txt(input: &str) -> Result<IndexRanges> {
    let mut result = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if let Some(space_index) = line.find(' ') {
            let (range_str, name) = line.split_at(space_index);
            result.push((name.trim().to_string(), parse_range(range_str)?));
        }
    }

    Ok(result)
}

/// Parse JSON file with format
/// ```json
/// {
///   "0..1000": "entityA lighthouse-geth-0",
///   "1000..2000": "entityB lodestar-nethermind-0",
/// }
/// ```
fn parse_ranges_as_json(input: &str) -> Result<IndexRanges> {
    let data: IndexRangesJson = serde_json::from_str(input)?;
    let mut result = Vec::new();
    for (range_str, name) in data {
        result.push((name, parse_range(&range_str)?));
    }
    Ok(result)
}

/// Parses a string representing a range with format:
/// "0-10", "0..10", "[0..10]", "[0-10]", "(0..10)", "[0-10)",
fn parse_range(input: &str) -> Result<Range<usize>> {
    let re = Regex::new(r"(\d+)[-.]+(\d+)")?;
    let captures = re
        .captures(input)
        .ok_or_else(|| anyhow!("Invalid range format: {}", input))?;
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
    fn parse_ranges_file_txt_test() {
        assert_eq!(
            parse_ranges(
                "
  0..100   entityA lighthouse-geth
  100..200 entityB lodestar-nethermind-1

",
            )
            .unwrap(),
            vec![
                ("entityA lighthouse-geth".to_owned(), 0..100),
                ("entityB lodestar-nethermind-1".to_owned(), 100..200),
            ]
        );
    }

    #[test]
    fn parse_ranges_file_json_test() {
        assert_eq!(
            parse_ranges(
                "{\"0..100\": \"entityA lighthouse-geth\", \"100..200\": \"entityB lodestar-nethermind-1\"}"
            )
            .unwrap(),
            vec![
                ("entityA lighthouse-geth".to_owned(), 0..100),
                ("entityB lodestar-nethermind-1".to_owned(), 100..200),
            ]
        );
    }

    #[test]
    fn parse_ranges_file_yaml_test() {
        assert_eq!(
            parse_ranges(
                "0..100: entityA lighthouse-geth
100..200: entityB lodestar-nethermind-1
",
            )
            .unwrap(),
            vec![
                ("entityA lighthouse-geth".to_owned(), 0..100),
                ("entityB lodestar-nethermind-1".to_owned(), 100..200),
            ]
        );
    }
}
