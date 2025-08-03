use bytes::{Buf, Bytes};
use miette::{Context, IntoDiagnostic, bail};
use serde_bytes::ByteBuf;
use uuid::Uuid;

use crate::Epoch;

#[derive(Clone, Debug)]
pub enum NGMessage {
    StateSubmitting(Epoch),
    SubmitName(String),
    NameSubmitted(String, Uuid),
    UnsubmitName(Uuid),
    NameUnsubmitted(Uuid),
    NumNames(usize),
    RequestPlayingState,
    Names(Vec<String>, Vec<u8>),
    GuessName(usize),
    NameGuessed(usize),
    UnguessName(usize),
    NameUnguessed(usize),
    RequestSubmittingState,
}

impl NGMessage {
    pub fn parse(mut bytes: Bytes) -> miette::Result<Self> {
        let typ = bytes.get_u32();
        match typ {
            0 => Ok(NGMessage::StateSubmitting(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from StateSubmitting message")?,
            )),
            1 => Ok(NGMessage::SubmitName(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from SubmitName message")?,
            )),
            2 => {
                let (name, id) = rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NameSubmitted message")?;
                Ok(NGMessage::NameSubmitted(name, id))
            }
            3 => Ok(NGMessage::UnsubmitName(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from UnsubmitName message")?,
            )),
            4 => Ok(NGMessage::NameUnsubmitted(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NameUnsubmitted message")?,
            )),
            5 => Ok(NGMessage::NumNames(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NumNames message")?,
            )),
            6 => {
                if bytes.len() != 0 {
                    bail!(
                        "nonzero length in RequestPlayingState message: {}",
                        bytes.len()
                    );
                } else {
                    Ok(NGMessage::RequestPlayingState)
                }
            }
            7 => {
                let (names, guesses): (Vec<String>, ByteBuf) = rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from Names message")?;
                Ok(NGMessage::Names(names, guesses.into_vec()))
            }
            8 => Ok(NGMessage::GuessName(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from GuessName message")?,
            )),
            9 => Ok(NGMessage::NameGuessed(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NameGuessed message")?,
            )),
            10 => Ok(NGMessage::UnguessName(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from UnguessName message")?,
            )),
            11 => Ok(NGMessage::NameUnguessed(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NameUnguessed message")?,
            )),
            12 => {
                if bytes.len() != 0 {
                    bail!(
                        "nonzero length in RequestSubmittingState message: {}",
                        bytes.len()
                    );
                } else {
                    Ok(NGMessage::RequestSubmittingState)
                }
            }
            _ => {
                bail!("message has unknown type: {typ}");
            }
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut encoded = vec![0; 4];

        encoded[..4].copy_from_slice(
            &match self {
                NGMessage::StateSubmitting(_) => 0u32,
                NGMessage::SubmitName(_) => 1,
                NGMessage::NameSubmitted(_, _) => 2,
                NGMessage::UnsubmitName(_) => 3,
                NGMessage::NameUnsubmitted(_) => 4,
                NGMessage::NumNames(_) => 5,
                NGMessage::RequestPlayingState => 6,
                NGMessage::Names(_, _) => 7,
                NGMessage::GuessName(_) => 8,
                NGMessage::NameGuessed(_) => 9,
                NGMessage::UnguessName(_) => 10,
                NGMessage::NameUnguessed(_) => 11,
                NGMessage::RequestSubmittingState => 12,
            }
            .to_be_bytes(),
        );

        match self {
            NGMessage::StateSubmitting(epoch) => {
                rmp_serde::encode::write(&mut encoded, epoch).unwrap()
            }
            NGMessage::SubmitName(name) => rmp_serde::encode::write(&mut encoded, name).unwrap(),
            NGMessage::NameSubmitted(name, id) => {
                rmp_serde::encode::write(&mut encoded, &(name, id)).unwrap()
            }
            NGMessage::UnsubmitName(id) => rmp_serde::encode::write(&mut encoded, id).unwrap(),
            NGMessage::NameUnsubmitted(id) => rmp_serde::encode::write(&mut encoded, id).unwrap(),
            NGMessage::NumNames(num) => rmp_serde::encode::write(&mut encoded, num).unwrap(),
            NGMessage::RequestPlayingState => {}
            NGMessage::Names(names, guesses) => {
                rmp_serde::encode::write(&mut encoded, &(names, serde_bytes::Bytes::new(&guesses)))
                    .unwrap()
            }
            NGMessage::GuessName(index) => rmp_serde::encode::write(&mut encoded, index).unwrap(),
            NGMessage::NameGuessed(index) => rmp_serde::encode::write(&mut encoded, index).unwrap(),
            NGMessage::UnguessName(index) => rmp_serde::encode::write(&mut encoded, index).unwrap(),
            NGMessage::NameUnguessed(index) => {
                rmp_serde::encode::write(&mut encoded, index).unwrap()
            }
            NGMessage::RequestSubmittingState => {}
        }

        Bytes::from(encoded)
    }
}
