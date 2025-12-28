use std::collections::{BTreeMap, HashMap};

use std::path::PathBuf;

use crate::card::Card;
use crate::crud::CardStatsRow;
use crate::fsrs::calculate_recall;

#[derive(Debug, Default)]
pub struct CardStats {
    pub total_cards_in_db: i64,
    pub num_cards: i64,
    pub card_lifecycles: HashMap<CardLifeCycle, i64>,
    pub due_cards: i64,
    pub upcoming_week: BTreeMap<String, usize>,
    pub upcoming_month: i64,
    pub file_paths: HashMap<PathBuf, usize>,
    pub difficulty_histogram: Histogram<5>,
    pub retrievability_histogram: Histogram<5>,
}

#[derive(Debug, Clone)]
pub struct Histogram<const N: usize> {
    pub bins: [u32; N],
    count: u64,
    sum: f64,
}

impl<const N: usize> Default for Histogram<N> {
    #[inline]
    fn default() -> Self {
        Self {
            bins: [0; N],
            count: 0,
            sum: 0.0,
        }
    }
}
impl<const N: usize> Histogram<N> {
    pub fn update(&mut self, value: f64) {
        let v = value.clamp(0.0, 1.0);
        let mut idx = (v * N as f64) as usize;
        idx = idx.min(N - 1);
        self.bins[idx] += 1;
        self.count += 1;
        self.sum += value;
    }
    pub fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CardLifeCycle {
    New,
    Young,
    Mature,
}
const MATURE_INTERVAL: f64 = 21.0;

impl CardStats {
    // row is a Record
    pub fn update(&mut self, card: &Card, row: &CardStatsRow) {
        let review_count = row.review_count;
        let due_date = row.due_date;
        let interval = row.interval_raw.unwrap_or_default();
        let difficulty = row.difficulty.unwrap_or_default();
        let stability = row.stability.unwrap_or_default();
        let last_reviewed_at = row.last_reviewed_at;

        let now = chrono::Utc::now();
        let week_horizon = now + chrono::Duration::days(7);
        let month_horizon = now + chrono::Duration::days(30);
        *self.file_paths.entry(card.file_path.clone()).or_insert(0) += 1;

        let lifecycle = if review_count == 0 {
            CardLifeCycle::New
        } else if interval > MATURE_INTERVAL {
            CardLifeCycle::Mature
        } else {
            CardLifeCycle::Young
        };

        *self.card_lifecycles.entry(lifecycle).or_insert(0) += 1;

        match due_date {
            None => {
                self.due_cards += 1;
                let day = now.format("%Y-%m-%d").to_string();
                *self.upcoming_week.entry(day).or_insert(0) += 1;
                self.upcoming_month += 1;
            }
            Some(due_date) => {
                if due_date <= now {
                    self.due_cards += 1;
                    let day = now.format("%Y-%m-%d").to_string();
                    *self.upcoming_week.entry(day).or_insert(0) += 1;
                    self.upcoming_month += 1;
                } else {
                    if due_date <= week_horizon {
                        let day = due_date.format("%Y-%m-%d").to_string();
                        *self.upcoming_week.entry(day).or_insert(0) += 1;
                    }

                    if due_date <= month_horizon {
                        self.upcoming_month += 1;
                    }
                }
            }
        }
        self.difficulty_histogram.update(difficulty / 10.0);
        let Some(last_reviewed_at) = last_reviewed_at else {
            return;
        };

        let elapsed_days =
            now.signed_duration_since(last_reviewed_at).num_seconds() as f64 / 86_400.0;
        let retrievabiliity = calculate_recall(elapsed_days.max(0.0), stability);
        self.retrievability_histogram.update(retrievabiliity);
    }
}
