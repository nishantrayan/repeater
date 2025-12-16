use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::card::{Card, CardContent};

pub fn validate_file_can_be_card(path: String) -> Result<PathBuf> {
    let card_path = trim_line(&path).ok_or_else(|| anyhow!("Card path cannot be empty"))?;
    let card_path = PathBuf::from(card_path);
    if card_path.is_dir() {
        return Err(anyhow!(
            "Card path cannot be a directory: {}",
            card_path.display()
        ));
    }

    if !is_markdown(&card_path) {
        return Err(anyhow!(
            "Card path must be a markdown file: {}",
            card_path.display()
        ));
    }

    Ok(card_path)
}
pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn find_cloze_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;

    for (i, ch) in text.char_indices() {
        match ch {
            '[' if start.is_none() => start = Some(i),
            ']' if start.is_some() => {
                let s = start.take().unwrap();
                let e = i;
                ranges.push((s, e));
            }
            _ => {}
        }
    }

    ranges
}
pub fn trim_line(line: &str) -> Option<String> {
    let trimmed_line = line.trim().to_string();
    if trimmed_line.is_empty() {
        return None;
    }
    Some(trimmed_line)
}
fn parse_card_lines(contents: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut question = None;
    let mut answer = None;
    let mut cloze = None;

    for raw_line in contents.lines() {
        let line = match trim_line(raw_line) {
            Some(line) => line,
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("Q:") {
            question = trim_line(rest);
        } else if let Some(rest) = line.strip_prefix("A:") {
            answer = trim_line(rest);
        } else if let Some(rest) = line.strip_prefix("C:") {
            cloze = trim_line(rest);
        }
    }

    (question, answer, cloze)
}
pub fn content_to_card(
    card_path: &Path,
    contents: &str,
    file_start_idx: usize,
    file_end_idx: usize,
) -> Result<Card> {
    let (question, answer, cloze) = parse_card_lines(contents);

    let card_hash = get_card_hash(contents, file_start_idx, file_end_idx);
    if let (Some(q), Some(a)) = (question, answer) {
        let content = CardContent::Basic {
            question: q,
            answer: a,
        };
        Ok(Card {
            file_path: card_path.to_path_buf(),
            file_card_range: (file_start_idx, file_end_idx),
            content,
            card_hash,
        })
    } else if let Some(c) = cloze {
        let cloze_idxs = find_cloze_ranges(&c);
        if cloze_idxs.is_empty() {
            return Err(anyhow!("Card is a cloze but can't find cloze text in []"));
        }
        let content = CardContent::Cloze {
            text: c,
            start: cloze_idxs[0].0,
            end: cloze_idxs[0].1,
        };
        Ok(Card {
            file_path: card_path.to_path_buf(),
            file_card_range: (file_start_idx, file_end_idx),
            content,
            card_hash,
        })
    } else {
        Err(anyhow!("Unable to create card {}", contents))
    }
}

pub fn get_card_hash(content: &str, start_idx: usize, end_idx: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(content.as_bytes());
    hasher.update(format!("{}:{}", start_idx, end_idx).as_bytes());
    hasher.finalize().to_string()
}

pub fn cards_from_md(path: &Path) -> Result<Vec<Card>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut cards = Vec::new();
    let mut buffer = String::new();
    let mut start_idx = 0;
    let mut last_idx = 0;
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.starts_with("Q:") || line.starts_with("C:") {
            if !buffer.is_empty() {
                cards.push(content_to_card(path, &buffer, start_idx, idx)?);
                buffer.clear();
            }
            start_idx = idx;
        }
        buffer.push_str(&line);
        buffer.push('\n');
        last_idx = idx;
    }
    if !buffer.is_empty() {
        cards.push(content_to_card(path, &buffer, start_idx, last_idx + 1)?);
    }

    Ok(cards)
}
#[cfg(test)]
mod tests {
    use crate::utils::content_to_card;
    use std::path::PathBuf;

    use crate::card::CardContent;

    use super::cards_from_md;

    #[test]
    fn basic_qa() {
        let card_path = PathBuf::from("test.md");

        let card = content_to_card(&card_path, "", 1, 1);
        assert!(card.is_err());

        let card = content_to_card(&card_path, "what am i doing here", 1, 1);
        assert!(card.is_err());

        let content = "Q: what?\nA: yes\n\n";
        let card = content_to_card(&card_path, content, 1, 1);
        if let CardContent::Basic { question, answer } = &card.expect("should be basic").content {
            assert_eq!(question, "what?");
            assert_eq!(answer, "yes");
        } else {
            panic!("Expected CardContent::Basic");
        }

        let content = "Q: what?\nA: \n\n";
        let card = content_to_card(&card_path, content, 1, 1);
        assert!(card.is_err());
    }

    #[test]
    fn basic_cloze() {
        let card_path = PathBuf::from("test.md");

        let content = "C: ping? [pong]";
        let card = content_to_card(&card_path, content, 1, 1);
        if let CardContent::Cloze { text, start, end } = &card.expect("should be basic").content {
            assert_eq!(text, "ping? [pong]");
            assert_eq!(*start, 6_usize);
            assert_eq!(*end, 11_usize);
        } else {
            panic!("Expected CardContent::Cloze");
        }
    }

    #[test]
    fn test_file_capture() {
        let card_path = PathBuf::from("test_data/test.md");
        let cards = cards_from_md(&card_path).expect("should be ok");

        assert_eq!(cards.len(), 3);
    }
}
