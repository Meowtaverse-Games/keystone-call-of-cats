use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StageId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub id: StageId,
    pub title: String,
}
