pub fn calculate_score(kills: u32, time: u32) -> u32 {
    kills * 1000 / (time + 1)
}
