use super::super::structs::*;
use crate::common::{Fields, InternalWordAnnotation};
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct IntermediateExcerpt {
    pub query: String,
    pub entry_index: EntryIndex,
    pub score: Score,
    pub source: WordListSource,
    pub word_index: usize,
    pub internal_annotations: Vec<InternalWordAnnotation>,
    pub fields: Fields,
}

impl Ord for IntermediateExcerpt {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for IntermediateExcerpt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for IntermediateExcerpt {}

impl PartialEq for IntermediateExcerpt {
    fn eq(&self, other: &Self) -> bool {
        self.entry_index == other.entry_index
    }
}
