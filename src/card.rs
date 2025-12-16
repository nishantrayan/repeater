use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Card {
    pub file_path: PathBuf,
    pub file_card_range: (usize, usize),
    pub content: CardContent,
    pub card_hash: String,
    //pub file_hash: String,
}

#[derive(Clone, Debug)]
pub enum CardContent {
    Basic {
        question: String,
        answer: String,
    },
    Cloze {
        text: String,
        start: usize,
        end: usize,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum CardType {
    Basic,
    Cloze,
}
