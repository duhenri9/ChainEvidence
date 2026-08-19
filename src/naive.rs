use crate::models::Block;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct NaiveIndexer {
    material_state: BTreeMap<String, String>,
}

impl NaiveIndexer {
    pub fn apply(&mut self, block: &Block) {
        let mut events = block.events.clone();
        events.sort_by_key(|event| event.log_index);
        for event in events {
            self.material_state.insert(event.key, event.value);
        }
    }

    #[must_use]
    pub fn material_state(&self) -> BTreeMap<String, String> {
        self.material_state.clone()
    }
}
