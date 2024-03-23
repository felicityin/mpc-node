use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::task::{Context, Poll};
use std::time::SystemTime;

use anyhow::{Context as _, Result};
use futures::channel::mpsc;
use futures::{stream::SplitStream, Sink, StreamExt};
use futures_util::{stream::SplitSink, SinkExt};
use prost::Message as _;
use round_based::{
    Delivery, Incoming, MessageDestination, MessageType, MsgId, Outgoing, PartyIndex,
};
use serde::{de::DeserializeOwned, Serialize};
use surf::Url;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::message::SyncMsg;
use crate::protos;

pub async fn join_computation<M>(
    address: &str,
    room_id: &str,
    index: u16,
) -> Result<(
    TssDelivery<M>,
    JoinHandle<Result<(), anyhow::Error>>,
    JoinHandle<Result<(), anyhow::Error>>,
)>
where
    M: Clone + Send + 'static + Serialize + DeserializeOwned,
{
    let url = Url::parse(&format!("{}/{}", address, room_id))?;
    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    let (write_to_server, mut read_from_server) = ws_stream.split();

    // Block until all parties are online
    while let Some(Ok(msg)) = read_from_server.next().await {
        match msg {
            Message::Text(msg) => {
                if handle_sync_msg(msg) {
                    break;
                }
            }
            Message::Ping(_) => log::info!("ping"),
            _ => log::error!("block: invalid msg!"),
        }
    }

    let (tx_to_protocol, rx_from_protocol) = mpsc::channel(10);
    let (tx_to_preserver, rx_from_delivery) = mpsc::channel(10);

    let listenging = tokio::spawn(listening(index, read_from_server, tx_to_protocol));
    let sending = tokio::spawn(send_to_server(rx_from_delivery, write_to_server));

    let delivery = TssDelivery {
        incoming: rx_from_protocol,
        outgoing: TssOutgoing {
            party_idx:   index,
            sender:      tx_to_preserver,
            next_msg_id: NextMessageId::default(),
        },
    };
    Ok((delivery, listenging, sending))
}

async fn listening<M>(
    index: u16,
    mut read_from_server: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    mut tx_to_protocol: mpsc::Sender<Result<Incoming<M>, std::io::Error>>,
) -> Result<()>
where
    M: Clone + Send + 'static + Serialize + DeserializeOwned,
{
    while let Some(msg) = read_from_server.next().await {
        match msg {
            Ok(msg) => match msg {
                Message::Text(msg) => {
                    handle_sync_msg(msg);
                }
                Message::Binary(bin) => {
                    if let Err(e) = handle_bin_msg(index, bin, &mut tx_to_protocol).await {
                        log::error!("handle binary message error: {}", e);
                    }
                }
                Message::Ping(_) => log::info!("ping"),
                Message::Pong(_) => log::info!("pong"),
                Message::Close(_) => log::info!("close"),
                Message::Frame(_) => log::info!("frame"),
            },
            Err(e) => match e {
                tungstenite::Error::ConnectionClosed => log::error!("connection closed"),
                tungstenite::Error::AlreadyClosed => log::error!("already closed"),
                tungstenite::Error::WriteBufferFull(_) => log::error!("wirte buffer full"),
                _ => log::error!("listening: unknonw error"),
            },
        }
    }

    Ok(())
}

async fn handle_bin_msg<M>(
    index: u16,
    bin: Vec<u8>,
    tx: &mut mpsc::Sender<Result<Incoming<M>, std::io::Error>>,
) -> Result<()>
where
    M: Clone + Send + 'static + Serialize + DeserializeOwned,
{
    let envelope = protos::Envelope::decode(bin.as_slice()).context("failed to decode Envelope")?;
    let content =
        protos::Content::decode(envelope.content()).context("failed to decode Message")?;
    let msg =
        serde_json::from_slice::<M>(content.message()).context("failed to deserialize Msg")?;

    log::debug!(
        "msg from server, type: {}, sender: {}, receiver: {}",
        content.r#type().as_str_name(),
        content.sender(),
        content.receiver(),
    );
    if content.sender() as u16 == index
        || (content.r#type() == protos::content::Type::P2p && content.receiver() as u16 != index)
    {
        return Ok(());
    }

    let incoming = Incoming {
        id: content.id(),
        sender: content.sender() as u16,
        msg_type: if content.r#type() == protos::content::Type::P2p {
            MessageType::P2P
        } else {
            MessageType::Broadcast
        },
        msg,
    };

    tx.send(Ok(incoming)).await?;
    Ok(())
}

fn handle_sync_msg(msg: String) -> bool {
    match serde_json::from_str::<SyncMsg>(&msg) {
        Ok(msg) => match msg.type_ {
            crate::message::SyncType::Join => log::info!("{} joined!", msg.device),
            crate::message::SyncType::Leave => log::info!("{} leaved!", msg.device),
            crate::message::SyncType::GenUuid => log::info!("your id is {}", msg.device),
            crate::message::SyncType::Start => {
                log::info!("start!");
                return true;
            }
        },
        Err(e) => log::error!("failed to deserialize a sync message: {}", e),
    }
    false
}

async fn send_to_server<M>(
    mut rx_from_delivery: mpsc::Receiver<Outgoing<Incoming<M>>>,
    mut write_to_server: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) -> Result<()>
where
    M: Clone + Send + 'static + Serialize + DeserializeOwned,
{
    while let Some(outgoing) = rx_from_delivery.next().await {
        log::debug!(
            "msg from delivery, sender: {}, id: {}, ",
            outgoing.msg.sender,
            outgoing.msg.id
        );

        let content = protos::Content {
            id: Some(outgoing.msg.id),
            r#type: Some(if outgoing.recipient == MessageDestination::AllParties {
                protos::content::Type::Broadcast.into()
            } else {
                protos::content::Type::P2p.into()
            }),
            sender: Some(outgoing.msg.sender as u32),
            message: Some(
                serde_json::to_vec(&outgoing.msg.msg)
                    .context("failed to serialize outgoing msg")?,
            ),
            ..Default::default()
        };

        let msg = protos::Envelope {
            content: Some(content.encode_to_vec()),
            timestamp: Some(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .context("broken clocks")
                    .map_err(|e| anyhow::anyhow!("timestamp error: {}", e))?
                    .as_secs(),
            ),
            ..Default::default()
        };

        write_to_server
            .send(Message::Binary(msg.encode_to_vec()))
            .await;
    }
    Ok(())
}

pub struct TssDelivery<M> {
    incoming: mpsc::Receiver<Result<Incoming<M>, std::io::Error>>,
    outgoing: TssOutgoing<M>,
}

impl<M> Delivery<M> for TssDelivery<M>
where
    M: Clone + Send + Unpin + 'static,
{
    type Receive = mpsc::Receiver<Result<Incoming<M>, std::io::Error>>;
    type ReceiveError = std::io::Error;
    type Send = TssOutgoing<M>;
    type SendError = std::io::Error;

    fn split(self) -> (Self::Receive, Self::Send) {
        (self.incoming, self.outgoing)
    }
}

pub struct TssOutgoing<M> {
    party_idx:   PartyIndex,
    sender:      mpsc::Sender<Outgoing<Incoming<M>>>,
    next_msg_id: NextMessageId,
}

impl<M> Sink<Outgoing<M>> for TssOutgoing<M>
where
    M: Clone + Send + 'static,
{
    type Error = std::io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, msg: Outgoing<M>) -> Result<(), Self::Error> {
        let msg_type = match msg.recipient {
            MessageDestination::AllParties => MessageType::Broadcast,
            MessageDestination::OneParty(_) => MessageType::P2P,
        };

        let id = self.next_msg_id.next();
        let sender = self.party_idx;

        self.sender
            .try_send(msg.map(|m| Incoming {
                id,
                sender,
                msg_type,
                msg: m,
            }))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct NextMessageId(AtomicU64);

impl NextMessageId {
    pub fn next(&self) -> MsgId {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
