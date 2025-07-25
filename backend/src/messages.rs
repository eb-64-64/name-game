use bytes::{Buf, Bytes};
use miette::{Context, IntoDiagnostic, bail};
use serde_bytes::ByteBuf;

#[derive(Clone, Debug)]
pub enum NGMessage {
    SubmitName(String),
    NumNames(usize),
    StatePlaying,
    Names(Vec<String>, Vec<u8>),
    MakeGuess(usize),
    NameGuessed(usize),
    StateSubmitting,
}

impl NGMessage {
    pub fn parse(mut bytes: Bytes) -> miette::Result<Self> {
        let typ = bytes.get_u32();
        match typ {
            0 => Ok(NGMessage::SubmitName(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from SubmitName message")?,
            )),
            1 => Ok(NGMessage::NumNames(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NumNames message")?,
            )),
            2 => {
                if bytes.len() != 0 {
                    bail!("nonzero length in StatePlaying message: {}", bytes.len());
                } else {
                    Ok(NGMessage::StatePlaying)
                }
            }
            3 => {
                let (names, guesses): (Vec<String>, ByteBuf) = rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from Names message")?;
                Ok(NGMessage::Names(names, guesses.into_vec()))
            }
            4 => Ok(NGMessage::MakeGuess(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from MakeGuess message")?,
            )),
            5 => Ok(NGMessage::NameGuessed(
                rmp_serde::from_slice(&bytes)
                    .into_diagnostic()
                    .wrap_err("parse content from NameGuessed message")?,
            )),
            6 => {
                if bytes.len() != 0 {
                    bail!("nonzero length in StateSubmitting message: {}", bytes.len());
                } else {
                    Ok(NGMessage::StateSubmitting)
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
                NGMessage::SubmitName(_) => 0u32,
                NGMessage::NumNames(_) => 1,
                NGMessage::StatePlaying => 2,
                NGMessage::Names(_, _) => 3,
                NGMessage::MakeGuess(_) => 4,
                NGMessage::NameGuessed(_) => 5,
                NGMessage::StateSubmitting => 6,
            }
            .to_be_bytes(),
        );

        match self {
            NGMessage::SubmitName(name) => rmp_serde::encode::write(&mut encoded, name).unwrap(),
            NGMessage::NumNames(num) => rmp_serde::encode::write(&mut encoded, num).unwrap(),
            NGMessage::Names(names, guesses) => {
                rmp_serde::encode::write(&mut encoded, &(names, serde_bytes::Bytes::new(&guesses)))
                    .unwrap()
            }
            NGMessage::MakeGuess(index) => rmp_serde::encode::write(&mut encoded, index).unwrap(),
            NGMessage::NameGuessed(index) => rmp_serde::encode::write(&mut encoded, index).unwrap(),
            _ => {}
        }

        Bytes::from(encoded)
    }
}
