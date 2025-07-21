use bytes::{Buf, Bytes};
use miette::{Context, IntoDiagnostic, bail};

#[derive(Clone, Debug)]
pub enum NGMessage {
    Submitting,
    Name(String),
    NumNames(usize),
    NotSubmitting,
    Names(Vec<String>),
}

impl NGMessage {
    pub fn parse(mut bytes: Bytes) -> miette::Result<Self> {
        let typ = bytes.get_u32();
        let len = bytes.get_u32();
        match typ {
            0 => {
                if len != 0 {
                    bail!("nonzero length in `Submitting` message: {len}");
                } else {
                    Ok(NGMessage::Submitting)
                }
            }
            1 => Ok(NGMessage::Name(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from Name message")?,
            )),
            2 => Ok(NGMessage::NumNames(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NumNames message")?,
            )),
            3 => {
                if len != 0 {
                    bail!("nonzero length in NotSubmitting message: {len}");
                } else {
                    Ok(NGMessage::NotSubmitting)
                }
            }
            4 => Ok(NGMessage::Names(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from Names message")?,
            )),
            _ => {
                bail!("message has unknown type: {typ}");
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
