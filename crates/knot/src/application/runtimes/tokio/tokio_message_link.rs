// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use tokio::sync::mpsc;

use crate::application::MessageLink;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokioMessageLinkError;

/// One endpoint of a bounded, in-process Tokio message link.
pub struct TokioMessageLink<Message> {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}

pub fn local_tokio_message_link<Message>(
    capacity: usize,
) -> (TokioMessageLink<Message>, TokioMessageLink<Message>) {
    let (left_tx, left_rx) = mpsc::channel(capacity);
    let (right_tx, right_rx) = mpsc::channel(capacity);

    (
        TokioMessageLink {
            tx: left_tx,
            rx: right_rx,
        },
        TokioMessageLink {
            tx: right_tx,
            rx: left_rx,
        },
    )
}

impl<Message> MessageLink<Message> for TokioMessageLink<Message>
where
    Message: Send,
{
    type Error = TokioMessageLinkError;

    async fn send(&mut self, message: Message) -> Result<(), Self::Error> {
        self.tx
            .send(message)
            .await
            .map_err(|_| TokioMessageLinkError)
    }

    async fn receive(&mut self) -> Result<Option<Message>, Self::Error> {
        Ok(self.rx.recv().await)
    }
}
