pub trait ScoreRepo {
    fn save(&self, score: u32);
    fn load(&self) -> u32;
}

pub enum GameEvent {
    // Define events here
}

pub trait EventPublisher {
    fn publish(&mut self, event: GameEvent);
}
