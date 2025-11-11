use serde::{Deserialize, Serialize};

/// Player's progression through stages.
/// Unlocked range is inclusive from 0..=unlocked_until.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageProgress {
    unlocked_until: usize,
}

impl StageProgress {
    /// Returns true if the stage index is unlocked (<= current unlocked_until).
    pub fn is_unlocked(&self, index: usize) -> bool {
        index <= self.unlocked_until
    }

    /// Unlock all stages up to `index` (inclusive). Returns true if state changed.
    pub fn unlock_until(&mut self, index: usize) -> bool {
        if index > self.unlocked_until {
            self.unlocked_until = index;
            true
        } else {
            false
        }
    }

    /// Total number of unlocked slots (stages) = unlocked_until + 1 (stage 0 always counts).
    pub fn unlocked_slots(&self) -> usize {
        self.unlocked_until + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_first_stage() {
        let p = StageProgress::default();
        assert!(p.is_unlocked(0));
        assert_eq!(p.unlocked_slots(), 1);
    }

    #[test]
    fn unlocking_advances_range() {
        let mut p = StageProgress::default();
        assert!(p.unlock_until(2));
        assert!(p.is_unlocked(2));
        assert_eq!(p.unlocked_slots(), 3);
        // unlocking same or lower doesn't change
        assert!(!p.unlock_until(1));
        assert_eq!(p.unlocked_slots(), 3);
    }
}
