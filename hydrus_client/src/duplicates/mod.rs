pub mod auto_resolution;
pub mod merge_options;

pub use auto_resolution::{
    DuplicateComparator, DuplicatesAutoResolutionManager, DuplicatesAutoResolutionRule,
};
pub use merge_options::{
    DuplicateContentMergeOptions, DuplicatePairDecision, PotentialDuplicatePair,
    PotentialDuplicatesPairQueue,
};
