use crate::error::Result;
use crate::types::{GrinboxAddress, Slate};

pub trait GrinboxPublisher {
    fn post_slate(&self, slate: &Slate, to: &GrinboxAddress) -> Result<()>;
}
