use bevy::prelude::{Resource, warn};
use serde::{Deserialize, Serialize};

use crate::resources::file_storage::{FileError, FileStorage};

pub const STAGE_PROGRESS_FILE: &str = "stage_progress.ron";

/// Player's progression through stages.
/// Unlocked range is inclusive from 0..=unlocked_until.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
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

    pub fn load_or_default(storage: &dyn FileStorage) -> Self {
        match storage.load(STAGE_PROGRESS_FILE) {
            Ok(Some(bytes)) => ron::de::from_bytes(&bytes).unwrap_or_else(|err| {
                warn!("Failed to parse stage progress data: {err}");
                StageProgress::default()
            }),
            Ok(None) => StageProgress::default(),
            Err(err) => {
                warn!("Failed to load stage progress data: {err}");
                StageProgress::default()
            }
        }
    }

    pub fn persist(&self, storage: &dyn FileStorage) -> Result<(), FileError> {
        let serialized = ron::ser::to_string(self)
            .map_err(|err| FileError::Other(format!("serialize stage progress: {err}")))?;
        storage
            .save(STAGE_PROGRESS_FILE, serialized.as_bytes())
            .map_err(|err| {
                warn!("Failed to save stage progress: {err}");
                err
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_first_stage() {
        let p = StageProgress::default();
        assert!(p.is_unlocked(0));
    }

    #[test]
    fn unlocking_advances_range() {
        let mut p = StageProgress::default();
        assert!(p.unlock_until(2));
        assert!(p.is_unlocked(2));
        // unlocking same or lower doesn't change
        assert!(!p.unlock_until(1));
    }
}
