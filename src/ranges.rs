use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{collections::HashMap, ops::Range};

pub type IndexGroups = Vec<(String, Vec<usize>)>;
type IndexRangesJson = HashMap<String, String>;
type RangesNotGroup = Vec<(Range<usize>, String)>;

/// Parse group file contents flexibly, either as JSON first or then TXT
pub fn parse_ranges(input: &str) -> Result<IndexGroups> {
    let ranges = if let Ok(ranges) = parse_ranges_as_json(input) {
        ranges
    } else {
        parse_ranges_as_txt(input)?
    };

    let mut ranges_grouped = HashMap::new();
    for (range, s) in ranges {
        ranges_grouped.entry(s).or_insert(Vec::new()).push(range);
    }

    Ok(ranges_grouped
        .into_iter()
        .map(|(s, ranges)| {
            let mut indexes: Vec<_> = ranges.iter().flat_map(|r| r.clone()).collect();
            indexes.sort_unstable();
            indexes.dedup();
            (s, indexes)
        })
        .collect())
}

/// Parse a file contents defining group ranges with format:
/// - Index range in flexible format before first space
/// - Everything after first space is the group name str
/// ```txt
/// 0..1000 entityA lighthouse-geth-0
/// 1000..2000 entityB lodestar-nethermind-0
/// ```
fn parse_ranges_as_txt(input: &str) -> Result<RangesNotGroup> {
    let mut result = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if let Some(space_index) = line.find(' ') {
            let (range_str, name) = line.split_at(space_index);
            result.push((parse_range(range_str)?, name.trim().to_string()));
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
fn parse_ranges_as_json(input: &str) -> Result<RangesNotGroup> {
    let data: IndexRangesJson = serde_json::from_str(input)?;
    let mut result = Vec::new();
    for (range_str, name) in data {
        result.push((parse_range(&range_str)?, name));
    }
    // serde_json uses HashMap which does not preserve order. Enforce ascending index order
    result.sort_by_key(|(range, _)| range.start);
    Ok(result)
}

/// Parses a string representing a range with format:
/// "0-10", "0..10", "[0..10]", "[0-10]", "(0..10)", "[0-10)",
fn parse_range(input: &str) -> Result<Range<usize>> {
    static RE_MULTI: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)[-.]+(\d+)").unwrap());
    static RE_SINGLE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d+)[:]*$").unwrap());

    if let Some(c) = RE_MULTI.captures(input.trim()) {
        let start: usize = c[1].parse()?;
        let end: usize = c[2].parse()?;
        Ok(start..end)
    } else if let Some(c) = RE_SINGLE.captures(input.trim()) {
        let start: usize = c[1].parse()?;
        Ok(start..start + 1)
    } else {
        Err(anyhow!("Invalid range format: {}", input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// expand range
    fn er(r: Range<usize>) -> Vec<usize> {
        r.into_iter().collect()
    }

    fn parse_ranges_test(input: &str) -> IndexGroups {
        let mut groups = parse_ranges(input).unwrap();
        // Ensure stable order for assertion
        groups.sort_by_key(|(s, _)| s.to_owned());
        groups
    }

    #[test]
    fn parse_range_test() {
        for input in [
            "0-10", "0..10", "0..10:", "[0..10]:", "[0..10]", "[0-10]", "(0..10)", "[0-10)",
        ] {
            assert_eq!(parse_range(&input).unwrap(), 0..10);
        }
    }

    #[test]
    fn parse_range_single_test() {
        for input in ["10", "10:", " 10  ", " 10: "] {
            assert_eq!(parse_range(&input).unwrap(), 10..11);
        }
    }

    #[test]
    fn parse_ranges_file_txt_test() {
        assert_eq!(
            parse_ranges_test(
                "
  0..100   entityA lighthouse-geth
  100..200 entityB lodestar-nethermind-1

",
            ),
            vec![
                ("entityA lighthouse-geth".to_owned(), er(0..100)),
                ("entityB lodestar-nethermind-1".to_owned(), er(100..200)),
            ]
        );
    }

    #[test]
    fn parse_ranges_file_json_test() {
        assert_eq!(
            parse_ranges_test(
                "{\"0..100\": \"entityA lighthouse-geth\", \"100..200\": \"entityB lodestar-nethermind-1\"}"
            ),
            vec![
                ("entityA lighthouse-geth".to_owned(), er(0..100)),
                ("entityB lodestar-nethermind-1".to_owned(), er(100..200)),
            ]
        );
    }

    #[test]
    fn parse_ranges_file_yaml_test() {
        assert_eq!(
            parse_ranges_test(
                "0..100: entityA lighthouse-geth
100..200: entityB lodestar-nethermind-1
",
            ),
            vec![
                ("entityA lighthouse-geth".to_owned(), er(0..100)),
                ("entityB lodestar-nethermind-1".to_owned(), er(100..200)),
            ]
        );
    }

    #[test]
    fn parse_ranges_file_single_repeat_test() {
        assert_eq!(
            parse_ranges_test(
                "
50..55: entityA lighthouse-geth
57..58: entityA lighthouse-geth
60: entityA lighthouse-geth
70: entityA lighthouse-geth
100..200: entityB lodestar-nethermind-1
",
            ),
            vec![
                (
                    "entityA lighthouse-geth".to_owned(),
                    vec![50, 51, 52, 53, 54, 57, 60, 70]
                ),
                ("entityB lodestar-nethermind-1".to_owned(), er(100..200)),
            ]
        );
    }
}
