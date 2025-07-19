use bytes::{Buf, Bytes};
use tracing::error;

#[derive(Clone, Debug)]
pub enum NGMessage {
    Submitting,
    Name(String),
    NumNames(u8),
    NotSubmitting,
    Names(Vec<String>),
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
                    Some(NGMessage::Submitting)
                }
            }
            1 => Some(NGMessage::Name(rmp_serde::from_slice(&bytes).ok()?)),
            2 => Some(NGMessage::NumNames(rmp_serde::from_slice(&bytes).ok()?)),
            3 => {
                if len != 0 {
                    error!("got nonzero length for PlayTime message type: {len}");
                    None
                } else {
                    Some(NGMessage::NotSubmitting)
                }
            }
            4 => Some(NGMessage::Names(rmp_serde::from_slice(&bytes).ok()?)),
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
                NGMessage::Submitting => 0u32,
                NGMessage::Name(_) => 1,
                NGMessage::NumNames(_) => 2,
                NGMessage::NotSubmitting => 3,
                NGMessage::Names(_) => 4,
            }
            .to_be_bytes(),
        );

        match self {
            NGMessage::Name(submission) => {
                rmp_serde::encode::write(&mut encoded, &submission).unwrap();
            }
            NGMessage::NumNames(num) => {
                rmp_serde::encode::write(&mut encoded, &num).unwrap();
            }
            NGMessage::Names(submissions) => {
                rmp_serde::encode::write(&mut encoded, &submissions).unwrap();
            }
            _ => {}
        }

        let len = encoded.len() - 8;
        encoded[4..8].copy_from_slice(&(len as u32).to_be_bytes());

        Bytes::from(encoded)
    }
}
