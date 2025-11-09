use serde::{Deserialize, Serialize};

/// Tracks which stage index has been unlocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    /// Zero-based index of the highest unlocked stage.
    pub highest_unlocked_index: usize,
}

impl Default for StageProgress {
    fn default() -> Self {
        Self {
            highest_unlocked_index: 0,
        }
    }
}

impl StageProgress {
    pub fn new(highest_unlocked_index: usize) -> Self {
        Self {
            highest_unlocked_index,
        }
    }

    pub fn is_unlocked(&self, stage_index: usize) -> bool {
        stage_index <= self.highest_unlocked_index
    }

    /// Unlocks every stage up to and including `stage_index`.
    /// Returns true if the unlock state changed.
    pub fn unlock_until(&mut self, stage_index: usize) -> bool {
        if stage_index > self.highest_unlocked_index {
            self.highest_unlocked_index = stage_index;
            true
        } else {
            false
        }
    }

    pub fn unlocked_slots(&self) -> usize {
        self.highest_unlocked_index + 1
    }
}
