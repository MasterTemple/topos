use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RangePair<T>
    // where T: Copy + Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Serialize + DeserializeOwned
{
    pub start: T,
    pub end: T,
}

impl<T> RangePair<T>
    where T: Copy + Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Serialize + DeserializeOwned
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn from_point(point: T) -> Self {
        Self {
            start: point,
            end: point,
        }
    }
}
