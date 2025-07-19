use bytes::{Buf, Bytes};
use tracing::error;

#[derive(Clone, Debug)]
pub enum NGMessage {
    SubmissionTime,
    Submission(String),
    NumSubmissions(u8),
    PlayTime,
    SubmissionList(Vec<String>),
}

impl NGMessage {
    pub fn parse(mut bytes: Bytes) -> Option<Self> {
        let typ = bytes.get_u32();
        let len = bytes.get_u32();
        match typ {
            0 => {
                if len != 0 {
                    error!("got nonzero length for SubmissionTime message type: {len}");
                    None
                } else {
                    Some(NGMessage::SubmissionTime)
                }
            }
            1 => Some(NGMessage::Submission(rmp_serde::from_slice(&bytes).ok()?)),
            2 => Some(NGMessage::NumSubmissions(
                rmp_serde::from_slice(&bytes).ok()?,
            )),
            3 => {
                if len != 0 {
                    error!("got nonzero length for PlayTime message type: {len}");
                    None
                } else {
                    Some(NGMessage::PlayTime)
                }
            }
            4 => Some(NGMessage::SubmissionList(
                rmp_serde::from_slice(&bytes).ok()?,
            )),
            _ => {
                error!("got unknown type: {typ}");
                None
            }
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut encoded = vec![0; 8];

        encoded[..4].copy_from_slice(
            &match self {
                NGMessage::SubmissionTime => 0u32,
                NGMessage::Submission(_) => 1,
                NGMessage::NumSubmissions(_) => 2,
                NGMessage::PlayTime => 3,
                NGMessage::SubmissionList(_) => 4,
            }
            .to_be_bytes(),
        );

        match self {
            NGMessage::Submission(submission) => {
                rmp_serde::encode::write(&mut encoded, &submission).unwrap();
            }
            NGMessage::NumSubmissions(num) => {
                rmp_serde::encode::write(&mut encoded, &num).unwrap();
            }
            NGMessage::SubmissionList(submissions) => {
                rmp_serde::encode::write(&mut encoded, &submissions).unwrap();
            }
            _ => {}
        }

        let len = encoded.len() - 8;
        encoded[4..8].copy_from_slice(&(len as u32).to_be_bytes());

        Bytes::from(encoded)
    }
}
