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
    Type5,
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

        // Type 4: Move + Touched + IsEmpty + Place
        let mut type4 = type2;
        type4.insert("place".to_string());
        map.insert(StoneType::Type4, type4);

        // Type 5: Every command, and the only type that can Send + Receive
        let type5 = [
            "move",
            "sleep",
            "is_touched",
            "is_empty",
            "dig",
            "place",
            "send",
            "receive",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        map.insert(StoneType::Type5, type5);

        Self { map }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type4_has_only_its_explicit_place_capabilities() {
        let capabilities = StoneCapabilities::default();
        let type4 = capabilities
            .get_capabilities(StoneType::Type4)
            .expect("Type4 must not be unrestricted");

        assert_eq!(
            type4,
            &HashSet::from([
                "move".to_string(),
                "is_touched".to_string(),
                "is_empty".to_string(),
                "place".to_string(),
            ])
        );
        assert!(!type4.contains("dig"));
        assert!(!type4.contains("sleep"));
    }

    #[test]
    fn only_type5_can_send_and_receive() {
        let capabilities = StoneCapabilities::default();
        for stone_type in [
            StoneType::Type1,
            StoneType::Type2,
            StoneType::Type3,
            StoneType::Type4,
            StoneType::Type5,
        ] {
            let allowed = capabilities
                .get_capabilities(stone_type)
                .expect("every stone type must be restricted");
            let can_signal = stone_type == StoneType::Type5;
            assert_eq!(allowed.contains("send"), can_signal, "{stone_type:?}");
            assert_eq!(allowed.contains("receive"), can_signal, "{stone_type:?}");
        }

        let type5 = capabilities.get_capabilities(StoneType::Type5).unwrap();
        for command in ["move", "sleep", "is_touched", "is_empty", "dig", "place"] {
            assert!(type5.contains(command), "Type5 is missing {command}");
        }
    }
}

impl StoneCapabilities {
    pub fn get_capabilities(&self, stone_type: StoneType) -> Option<&HashSet<String>> {
        self.map.get(&stone_type)
    }
}
