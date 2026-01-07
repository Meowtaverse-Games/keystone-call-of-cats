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

        // Type 1: Move + Sleep
        let mut type1 = HashSet::new();
        type1.insert("move".to_string());
        type1.insert("sleep".to_string());
        map.insert(StoneType::Type1, type1);

        // Type 2: Move + Touched + IsEmpty
        let mut type2 = HashSet::new();
        type2.insert("move".to_string());
        type2.insert("is_touched".to_string());
        type2.insert("is_empty".to_string());
        map.insert(StoneType::Type2, type2.clone());

        // Type 3: Move + Sleep + Touched + Dig + IsEmpty
        let mut type3 = HashSet::new();
        type3.insert("move".to_string());
        type3.insert("sleep".to_string());
        type3.insert("is_touched".to_string());
        type3.insert("is_empty".to_string());
        type3.insert("dig".to_string());
        map.insert(StoneType::Type3, type3);

        // // Type 4: Move + Touched
        // let mut type4 = HashSet::new();
        // type4.insert("move".to_string());
        // type4.insert("is_touched".to_string());
        // map.insert(StoneType::Type4, type4);

        Self { map }
    }
}

impl StoneCapabilities {
    pub fn get_capabilities(&self, stone_type: StoneType) -> Option<&HashSet<String>> {
        self.map.get(&stone_type)
    }
}
