/// Frozen IDs of the released numeric stages. Never reorder or reuse.
pub const LEGACY_STAGE_IDS: [&str; 12] = [
    "move-01", "move-02", "move-03", "move-04", "move-05", "move-06", "sense-01", "sense-02",
    "dig-01", "dig-02", "dig-03", "dig-04",
];

pub fn legacy_stage_id(number: usize) -> Option<&'static str> {
    number
        .checked_sub(1)
        .and_then(|i| LEGACY_STAGE_IDS.get(i).copied())
}
