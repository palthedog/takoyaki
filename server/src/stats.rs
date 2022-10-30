use std::{
    collections::HashMap,
    fmt::Display,
};

use itertools::Itertools;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd)]
pub struct NamedScore {
    pub name: String,
    pub score: u32,
}

impl NamedScore {
    pub fn new(name: &str, score: u32) -> Self {
        Self {
            name: name.to_string(),
            score,
        }
    }
}

impl Display for NamedScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ({})", self.name, self.score)
    }
}

struct Stats {
    pub win: u32,
    pub draw: u32,
    pub lose: u32,
}

/// Stores game results.
pub struct StatsCounter {
    // pair (A v.s. B)
    counts: HashMap<(String, String), Stats>,

    // per player
    totals: HashMap<String, Stats>,
}

impl StatsCounter {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            totals: HashMap::new(),
        }
    }

    pub fn push_result(&mut self, a: &NamedScore, b: &NamedScore) {
        // We need a consistent player order.
        if a.name > b.name {
            self.push_result(b, a);
            return;
        }

        let key = (a.name.clone(), b.name.clone());
        let mut entry_pair = self.counts.entry(key).or_insert(Stats {
            win: 0,
            draw: 0,
            lose: 0,
        });
        let mut entry_total_0 = self.totals.entry(a.name.clone()).or_insert(Stats {
            win: 0,
            draw: 0,
            lose: 0,
        });
        match a.score.cmp(&b.score) {
            std::cmp::Ordering::Less => {
                entry_pair.lose += 1;
                entry_total_0.lose += 1;
            }
            std::cmp::Ordering::Equal => {
                entry_pair.draw += 1;
                entry_total_0.draw += 1;
            }
            std::cmp::Ordering::Greater => {
                entry_pair.win += 1;
                entry_total_0.win += 1;
            }
        };

        let mut entry_total_1 = self.totals.entry(b.name.clone()).or_insert(Stats {
            win: 0,
            draw: 0,
            lose: 0,
        });
        match a.score.cmp(&b.score) {
            std::cmp::Ordering::Less => {
                entry_total_1.win += 1;
            }
            std::cmp::Ordering::Equal => {
                entry_total_1.draw += 1;
            }
            std::cmp::Ordering::Greater => {
                entry_total_1.lose += 1;
            }
        }
    }
}

impl Default for StatsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for StatsCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StatsCounter {{")?;
        writeln!(f, "* Match results")?;
        for (k, v) in self.counts.iter() {
            writeln!(
                f,
                r#"  {:<20} vs {:<20} | {:<8}, {:<8}, draw: {:<4}"#,
                k.0, k.1, v.win, v.lose, v.draw
            )?;
        }
        writeln!(f, "* Win ratios")?;
        for (k, v) in self
            .totals
            .iter()
            .map(|(k, v)| {
                let ratio = v.win as f64 / (v.win + v.lose + v.draw) as f64;
                (k, ratio)
            })
            .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
        {
            writeln!(f, r#"  {:<20} | {:.3}"#, k, v)?;
        }

        Ok(())
    }
}
