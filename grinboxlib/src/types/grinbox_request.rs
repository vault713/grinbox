use colored::*;
use std::fmt::{Display, Formatter, Result};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GrinboxRequest {
    Challenge,
    Subscribe {
        address: String,
        signature: String,
    },
    PostSlate {
        from: String,
        to: String,
        str: String,
        signature: String,
        message_expiration_in_seconds: Option<u32>,
    },
    Unsubscribe {
        address: String,
    },
}

impl Display for GrinboxRequest {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            GrinboxRequest::Challenge => write!(f, "{}", "Challenge".bright_purple()),
            GrinboxRequest::Subscribe {
                ref address,
                signature: _,
            } => write!(
                f,
                "{} to {}",
                "Subscribe".bright_purple(),
                address.bright_green()
            ),
            GrinboxRequest::Unsubscribe { ref address } => write!(
                f,
                "{} from {}",
                "Unsubscribe".bright_purple(),
                address.bright_green()
            ),
            GrinboxRequest::PostSlate {
                ref from,
                ref to,
                str: _,
                signature: _,
                message_expiration_in_seconds: _,
            } => write!(
                f,
                "{} from {} to {}",
                "PostSlate".bright_purple(),
                from.bright_green(),
                to.bright_green()
            ),
        }
    }
}