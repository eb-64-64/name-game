use axum::extract::ws::{Message, WebSocket};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use miette::{Context, IntoDiagnostic, bail};

use crate::messages::NGMessage;

pub struct Socket {
    sender: Sender,
    receiver: Receiver,
}

impl Socket {
    pub fn new(socket: WebSocket) -> Self {
        let (sender, receiver) = socket.split();
        let sender = Sender::new(sender);
        let receiver = Receiver::new(receiver);
        Self { sender, receiver }
    }

    pub async fn send(&mut self, message: NGMessage) -> miette::Result<()> {
        self.sender.send(message).await
    }

    #[allow(dead_code)]
    pub async fn recv(&mut self) -> miette::Result<Option<NGMessage>> {
        self.receiver.recv().await
    }

    pub fn split(self) -> (Sender, Receiver) {
        (self.sender, self.receiver)
    }
}

pub struct Sender {
    sender: SplitSink<WebSocket, Message>,
}

impl Sender {
    fn new(sender: SplitSink<WebSocket, Message>) -> Self {
        Self { sender }
    }

    pub async fn send(&mut self, message: NGMessage) -> miette::Result<()> {
        self.sender
            .send(Message::Binary(message.encode()))
            .await
            .into_diagnostic()
    }
}

pub struct Receiver {
    receiver: SplitStream<WebSocket>,
}

impl Receiver {
    fn new(receiver: SplitStream<WebSocket>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> miette::Result<Option<NGMessage>> {
        let bytes = match self.receiver.next().await {
            Some(Ok(Message::Close(_))) | None => return Ok(None),
            Some(Ok(Message::Binary(bytes))) => bytes,
            Some(Ok(message)) => bail!("got non-binary message from client: {message:?}"),
            Some(Err(err)) => {
                return Err(err)
                    .into_diagnostic()
                    .wrap_err("got error while receiving message from client");
            }
        };
        Ok(Some(
            NGMessage::parse(bytes).wrap_err("parse message from client")?,
        ))
    }
}
