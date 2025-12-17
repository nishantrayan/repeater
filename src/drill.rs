use std::path::Path;

use crate::card::Card;
use crate::crud::DB;
use crate::utils::{cards_from_dir, cards_from_md};

use anyhow::Result;

pub async fn run(
    db: &DB,
    paths: Vec<String>,
    card_limit: Option<usize>,
    new_card_limit: Option<usize>,
) -> Result<()> {
    let cards = register_all_cards(db, paths).await?;
    let available_card_hashes: Vec<&str> = cards.iter().map(|c| c.card_hash.as_str()).collect();
    Ok(())
}

pub async fn register_all_cards(db: &DB, paths: Vec<String>) -> Result<Vec<Card>> {
    let mut all_cards = Vec::new();

    for path in paths {
        let p = Path::new(&path);

        if p.is_dir() {
            let mut cards = cards_from_dir(p)?;
            all_cards.append(&mut cards);
        } else if p.is_file() {
            let mut cards = cards_from_md(p)?;
            all_cards.append(&mut cards);
        } else {
            eprintln!("Warning: {path} does not exist or is not accessible.");
        }
    }
    db.add_cards_batch(&all_cards).await?;

    Ok(all_cards)
}
