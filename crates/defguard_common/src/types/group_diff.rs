use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct GroupDiff {
    pub added: HashSet<String>,
    pub removed: HashSet<String>,
}

impl GroupDiff {
    #[must_use]
    pub fn changed(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty()
    }
}
