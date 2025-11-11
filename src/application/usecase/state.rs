use bevy::prelude::Resource;
use std::sync::Arc;

use crate::application::ports::StageRepository;
use crate::application::usecase::stage_catalog_usecase::StageCardEntry;
use crate::application::usecase::stage_progress_usecase::StageProgressServiceRes;
use crate::domain::stage::{StageId, StageMeta};
use crate::domain::stage_progress::StageProgress;
use bevy::prelude::warn;

/// Cached stage catalog (metas + derived playable entries).
#[derive(Resource, Clone)]
pub struct StageCatalogRes {
    metas: Arc<[StageMeta]>,
    entries: Vec<StageCardEntry>,
}

impl StageCatalogRes {
    pub fn new(metas: Vec<StageMeta>, progress: &StageProgress) -> Self {
        let metas_arc: Arc<[StageMeta]> = metas.into();
        let entries = Self::build_entries(&metas_arc, progress);
        Self {
            metas: metas_arc,
            entries,
        }
    }

    fn build_entries(metas: &[StageMeta], progress: &StageProgress) -> Vec<StageCardEntry> {
        metas
            .iter()
            .map(|m| StageCardEntry {
                index: m.id.0,
                title: m.title.clone(),
                playable: progress.is_unlocked(m.id.0),
            })
            .collect()
    }

    pub fn entries(&self) -> &[StageCardEntry] {
        &self.entries
    }

    pub fn refresh_all(&mut self, progress: &StageProgress) {
        self.entries = Self::build_entries(&self.metas, progress);
    }

    pub fn mark_unlocked(&mut self, stage_index: usize) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.index == stage_index) {
            entry.playable = true;
        }
    }
}

/// Cached progress wrapper (in-memory). All mutations happen here first, then persisted.
#[derive(Resource, Clone)]
pub struct StageProgressRes(pub StageProgress);

impl StageProgressRes {
    pub fn unlock_until(&mut self, index: usize) -> bool {
        self.0.unlock_until(index)
    }
}

/// Initialize both catalog and progress resources from repository + progress service.
pub fn init_stage_state(
    repo: &dyn StageRepository,
    progress_service: &StageProgressServiceRes,
) -> (StageCatalogRes, StageProgressRes) {
    let progress = progress_service.load_or_default().unwrap_or_default();
    let metas = repo.list().unwrap_or_default();
    let catalog = StageCatalogRes::new(metas, &progress);
    let progress_res = StageProgressRes(progress);
    (catalog, progress_res)
}

/// Unlock flow: update progress + save + update catalog.
pub fn unlock_stage(
    progress_res: &mut StageProgressRes,
    catalog_res: &mut StageCatalogRes,
    progress_service: &StageProgressServiceRes,
    stage_index: usize,
) {
    if progress_res.unlock_until(stage_index) {
        // Persist
        if let Err(e) = progress_service.save(&progress_res.0) {
            warn!("Failed to save stage progress: {:?}", e);
        }
        // Update catalog view (diff)
        catalog_res.mark_unlocked(stage_index);
    }
}
