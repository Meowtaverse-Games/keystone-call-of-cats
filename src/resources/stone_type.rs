use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Component)]
pub enum StoneType {
    #[default]
    Type1,
    Type2,
    Type3,
    Type4,
}

#[derive(Resource)]
pub struct StoneCapabilities {
    map: HashMap<StoneType, HashSet<String>>,
}

impl Default for StoneCapabilities {
    fn default() -> Self {
        let mut map = HashMap::new();
        // Definition of capabilities for each type
        // Type 1: Basic (Move, Sleep)
        let mut type1 = HashSet::new();
        type1.insert("move".to_string());
        type1.insert("sleep".to_string());
        map.insert(StoneType::Type1, type1);

        // Type 2: + Touched ? (Example, to be refined based on specific requirements)
        let mut type2 = HashSet::new();
        type2.insert("move".to_string());
        type2.insert("sleep".to_string());
        type2.insert("touched".to_string());
        map.insert(StoneType::Type2, type2.clone());

        // Placeholder for Type 3 and 4, defaulting to same as Type 2 for now
        map.insert(StoneType::Type3, type2.clone());
        map.insert(StoneType::Type4, type2);

        Self { map }
    }
}

impl StoneCapabilities {
    pub fn get_capabilities(&self, stone_type: StoneType) -> Option<&HashSet<String>> {
        self.map.get(&stone_type)
    }
}
