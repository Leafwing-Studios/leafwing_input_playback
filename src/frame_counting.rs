//! Counts (and updates) the frame of the app

use bevy::ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};
/// The number of frames that have elapsed since the app started
///
/// Updated in [`time_tracker`] during [`CoreStage::First`].
#[derive(
    Resource,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct FrameCount(pub u64);

impl Add<FrameCount> for FrameCount {
    type Output = FrameCount;
    fn add(self, rhs: FrameCount) -> Self::Output {
        FrameCount(self.0.saturating_add(rhs.0))
    }
}

impl Sub<FrameCount> for FrameCount {
    type Output = FrameCount;
    fn sub(self, rhs: FrameCount) -> Self::Output {
        FrameCount(self.0.saturating_sub(rhs.0))
    }
}

/// A system which increases the value of the [`FrameCount`] resource by 1 every frame
///
/// This system should run during [`CoreStage::First`].
pub fn frame_counter(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 += 1;
}
