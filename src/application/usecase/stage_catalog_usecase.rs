use crate::application::ports::StageRepository;
use crate::domain::stage::{StageId, StageMeta};
use crate::domain::stage_progress::StageProgress;

#[derive(Debug, Clone)]
pub struct StageCardEntry {
    pub index: usize,
    pub title: String,
    pub playable: bool,
}

pub struct StageCatalogUseCase<'a, R: StageRepository> {
    repo: &'a R,
}

impl<'a, R: StageRepository> StageCatalogUseCase<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub fn list_cards(&self, progress: &StageProgress) -> Result<Vec<StageCardEntry>, String> {
        let metas = self.repo.list().map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(metas.len());
        for StageMeta {
            id: StageId(index),
            title,
        } in metas
        {
            out.push(StageCardEntry {
                index,
                title,
                playable: progress.is_unlocked(index),
            });
        }
        Ok(out)
    }

    // pub fn unlock_stage(&self, stage_index: usize) -> Result<StageProgress, StageProgressError> {
    //     let mut progress = self.load_or_default()?;
    //     if progress.unlock_until(stage_index) {
    //         self.save(&progress)?;
    //     }
    //     Ok(progress)
    // }
}
