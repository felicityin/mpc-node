use anyhow::{Context, Result};
use futures::channel::mpsc;
use futures::{stream::SplitStream, Sink, Stream, StreamExt, TryStreamExt};
use futures_util::SinkExt;
use prost::Message as _;
use round_based::Msg;
use serde::{de::DeserializeOwned, Serialize};
use surf::Url;
use tokio::net::TcpStream;
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
    u16,
    impl Stream<Item = Result<Msg<M>>>,
    impl Sink<Msg<M>, Error = anyhow::Error>,
)>
where
    M: Serialize + DeserializeOwned,
{
    let url = Url::parse(&format!("{}/{}", address, room_id))?;
    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    let (write, mut read) = ws_stream.split();

    let (incoming_tx, incoming_rx) = mpsc::channel(10);

    // Block until all parties are online
    while let Some(Ok(msg)) = read.next().await {
        match msg {
            Message::Text(msg) => {
                println!("text");

                if handle_sync_msg(msg) {
                    break;
                }
            }
            Message::Ping(_) => println!("ping"),
            _ => println!("invalid msg!"),
        }
    }

    let _ = tokio::spawn(listening(read, incoming_tx));

    let incoming = incoming_rx.and_then(move |msg| {
        Box::pin(async move {
            let msg: Msg<M> = serde_json::from_slice(msg.content())
                .context("failed to deserialize a message")?;
            Ok(msg)
        })
    });

    let incoming = incoming.try_filter(move |msg| {
        futures::future::ready(
            msg.sender != index && (msg.receiver.is_none() || msg.receiver == Some(index)),
        )
    });

    // Construct channel of outgoing messages
    let outgoing = futures::sink::unfold(write, |mut write, message: Msg<M>| async move {
        let serialized = serde_json::to_vec(&message).context("serialize message")?;
        let msg = protos::Envelope {
            content: Some(serialized),
            ..Default::default()
        };

        write.send(Message::Binary(msg.encode_to_vec())).await;
        Ok::<_, anyhow::Error>(write)
    });

    Ok((index, incoming, outgoing))
}

async fn listening(
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    mut tx: mpsc::Sender<Result<protos::Envelope, anyhow::Error>>,
) {
    while let Some(msg) = read.next().await {
        match msg.unwrap() {
            Message::Text(msg) => {
                println!("text");

                handle_sync_msg(msg);
            }
            Message::Binary(msg) => {
                println!("listen binary");

                let msg: Result<protos::Envelope, prost::DecodeError> =
                    protos::Envelope::decode(msg.as_slice());
                if let Err(e) = msg {
                    println!("failed to parse Envelope message: {}", e);
                    continue;
                }

                let msg = msg.unwrap();
                tx.send(Ok(msg)).await;
            }
            Message::Ping(_) => {
                println!("ping");
            }
            Message::Pong(_) => {
                println!("pong");
            }
            Message::Close(_) => {
                println!("close");
            }
            Message::Frame(_) => {
                println!("frame");
            }
        }
    }
}

fn handle_sync_msg(msg: String) -> bool {
    let msg: serde_json::Result<SyncMsg> = serde_json::from_str(&msg);
    if let Err(e) = msg {
        println!("failed to deserialize a sync message: {}", e);
        return true;
    }

    let msg = msg.unwrap();
    match msg.type_ {
        crate::message::SyncType::Join => {
            println!("{} joined!", msg.device);
        }
        crate::message::SyncType::Leave => {
            println!("{} leaved!", msg.device);
        }
        crate::message::SyncType::GenUuid => {
            println!("your id is {}", msg.device);
        }
        crate::message::SyncType::Start => {
            println!("start!");
            return true;
        }
    }
    false
}
