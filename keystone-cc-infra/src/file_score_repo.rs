use bevy::prelude::Resource;
use keystone_cc_core::boundary::ScoreRepo;

#[derive(Resource)]
pub struct FileScoreRepo;

impl ScoreRepo for FileScoreRepo {
    fn save(&self, score: u32) {
        // TODO: Implement file save
        println!("Saving score: {}", score);
    }

    fn load(&self) -> u32 {
        // TODO: Implement file load
        0
    }
}
