use crate::error::Error;

pub enum CloseReason {
    Normal,
    Abnormal(Error),
}
