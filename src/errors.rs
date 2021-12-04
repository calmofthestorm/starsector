use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum StructureError {
    IndextreeError(indextree::NodeError),
    LevelError,
}

#[cfg(feature = "headline-parser")]
#[derive(Debug, Clone)]
pub enum HeadlineError {
    NonEquivalentReparseError,
    InvalidTagsError,
    InvalidPriorityError,
    InvalidBodyError,
    InvalidLevelError,
    InvalidKeywordError,
    InvalidHeadlineError,
}

impl Display for StructureError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            StructureError::IndextreeError(e) => e.fmt(f),
            StructureError::LevelError => f.write_str("LevelError"),
        }
    }
}

impl Error for StructureError {
    fn description(&self) -> &str {
        match self {
            StructureError::IndextreeError(e) => e.description(),
            StructureError::LevelError => "LevelError",
        }
    }
}

impl From<indextree::NodeError> for StructureError {
    fn from(e: indextree::NodeError) -> StructureError {
        StructureError::IndextreeError(e)
    }
}

#[cfg(feature = "headline-parser")]
impl Display for HeadlineError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            HeadlineError::NonEquivalentReparseError => f.write_str("NonEquivalentReparseError"),
            HeadlineError::InvalidTagsError => f.write_str("InvalidTagsError"),
            HeadlineError::InvalidPriorityError => f.write_str("InvalidPriorityError"),
            HeadlineError::InvalidBodyError => f.write_str("InvalidBodyError"),
            HeadlineError::InvalidLevelError => f.write_str("InvalidLevelError"),
            HeadlineError::InvalidKeywordError => f.write_str("InvalidKeywordError"),
            HeadlineError::InvalidHeadlineError => f.write_str("InvalidHeadlineError"),
        }
    }
}

#[cfg(feature = "headline-parser")]
impl Error for HeadlineError {
    fn description(&self) -> &str {
        match self {
            HeadlineError::NonEquivalentReparseError => "NonEquivalentReparseError",
            HeadlineError::InvalidTagsError => "InvalidTagsError",
            HeadlineError::InvalidPriorityError => "InvalidPriorityError",
            HeadlineError::InvalidBodyError => "InvalidBodyError",
            HeadlineError::InvalidLevelError => "InvalidLevelError",
            HeadlineError::InvalidKeywordError => "InvalidKeywordError",
            HeadlineError::InvalidHeadlineError => "InvalidHeadlineError",
        }
    }
}
