use crate::model_download::ModelStatus;

pub const MODEL_NAME: &str = "OpenThai";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelChip {
    Ready,
    Loading,
    Downloading { percent: u64 },
    Verifying,
    Missing,
}

pub fn model_chip(status: &ModelStatus, loading: bool) -> ModelChip {
    match status {
        ModelStatus::Ready if loading => ModelChip::Loading,
        ModelStatus::Ready => ModelChip::Ready,
        ModelStatus::Downloading { done, total } => ModelChip::Downloading {
            percent: (done * 100).checked_div(*total).unwrap_or(0).min(100),
        },
        ModelStatus::Verifying => ModelChip::Verifying,
        ModelStatus::Failed(_) => ModelChip::Missing,
    }
}
