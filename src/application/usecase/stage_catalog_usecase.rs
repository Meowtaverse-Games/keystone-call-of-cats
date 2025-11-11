use crate::application::ports::StageRepository;
use crate::domain::stage::{StageId, StageMeta};
use crate::domain::stage_progress::StageProgress;

#[derive(Debug, Clone)]
pub struct StageCardEntry {
    pub index: usize,
    pub title: String,
    pub playable: bool,
}

#[allow(dead_code)]
pub struct StageCatalogUseCase<'a, R: StageRepository + ?Sized> {
    repo: &'a R,
}

impl<'a, R: StageRepository + ?Sized> StageCatalogUseCase<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub fn list_stage_cards(
        &self,
        progress: &StageProgress,
    ) -> Result<Vec<StageCardEntry>, String> {
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
}