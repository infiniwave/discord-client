use std::sync::mpsc;
// use std::sync::Arc;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

use async_native_tls::TlsStream;
use async_std::net::TcpStream;
// use async_std::sync::Mutex;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::stream::Stream;
use async_tungstenite::tungstenite::Message;
use async_tungstenite::tungstenite::Message::Text;
use async_tungstenite::WebSocketStream;
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
// use futures_util::FutureExt;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde::Deserialize;
use serde::Serialize;

enum ThreadEvent {
    SendMessage(String),
    Abort,
}

// #[derive(Debug, Deserialize, Serialize)]
// struct DiscordEvent {
//     op: u8,
//     s: Option<u64>,
//     #[serde(flatten)]
//     payload: DiscordPayload
// }

// #[derive(Debug, Deserialize, Serialize)]
// #[serde(tag = "t", content = "d")]
// enum DiscordPayload {
//     #[serde(rename = "READY")]
//     Ready {
//         v: u8,
//         user: DiscordUser,
//         session_id: String,
//         guilds: Vec<DiscordGuild>,
//         shard: Option<Vec<u8>>,
//     },
//     #[serde(rename = "MESSAGE_CREATE")]
//     MessageCreate {
//         id: String,
//         channel_id: String,
//         guild_id: Option<String>,

//     },
// }


#[derive(Debug, Serialize, Deserialize)]
struct HelloPayload {
    op: u8,
    d: HelloData,
}

#[derive(Debug, Serialize, Deserialize)]
struct HelloData {
    heartbeat_interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdentifyPayload {
    op: u8,
    d: IdentifyData,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdentifyData {
    token: String,
    intents: u64,
    properties: IdentifyProperties,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdentifyProperties {
    #[serde(rename = "$os")]
    os: String,
    #[serde(rename = "$browser")]
    browser: String,
    #[serde(rename = "$device")]
    device: String,
}

pub struct GatewayClient {
    token: String,
    heartbeat_interval: u64,
    receiver: Option<SplitStream<WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>>>,
    thread_sender: Option<mpsc::Sender<ThreadEvent>>,
}

impl GatewayClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            heartbeat_interval: 0,
            thread_sender: None,
            receiver: None,
        }
    }

    pub async fn start(&mut self) {
        let (ws_stream, _) = connect_async("wss://gateway.discord.gg/?v=9&encoding=json")
            .await
            .unwrap();
        let (write, mut read) = ws_stream.split();
        let hello = read.next().await.unwrap().unwrap();
        if let Text(hello) = hello {
            let json = serde_json::from_str::<HelloPayload>(&hello).unwrap();
            self.heartbeat_interval = json.d.heartbeat_interval;
        } else {
            panic!("Expected hello message");
        }

        let (tx, rx) = mpsc::channel();
        let interval = self.heartbeat_interval.clone();
        spawn(move || socket(interval, write, rx));
        self.receiver = Some(read);
        self.thread_sender = Some(tx);

        let identify = serde_json::json!({
            "op": 2,
            "d": {
                "token": self.token,
                "properties": {
                    "$os": "linux",
                    "$browser": "discord-rs",
                    "$device": "discord-rs",
                },
            }
        });
        self.thread_sender
            .as_ref()
            .unwrap()
            .send(ThreadEvent::SendMessage(
                serde_json::to_string(&identify).unwrap(),
            ))
            .unwrap();
        loop {
            let message = self.receiver.as_mut().unwrap().next().await;
            if let Some(message) = message {
                if let Text(message) = message.unwrap() {
                    println!("{}", message);
                }
            }
        }
    }

    // pub fn get_message(&mut self) -> Option<Message> {
    //     if let Some(receiver) = &mut self.receiver {
    //         if let Some(message) = receiver.next().now_or_never().unwrap() {
    //             return Some(message.unwrap());
    //         }
    //     }
    //     None
    // }
}

fn socket(
    interval: u64,
    mut socket_sender: SplitSink<WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>, Message>,
    thread_receiver: mpsc::Receiver<ThreadEvent>,
) {
    async_std::task::block_on(async move {
        'outer: loop {
            loop {
                match thread_receiver.try_recv() {
                    Ok(ThreadEvent::SendMessage(val)) => {
                        match socket_sender.send(Text(val)).await {
                            Ok(()) => (),
                            Err(why) => println!("Error sending gateway message: {:?}", why),
                        }
                    }
                    Ok(ThreadEvent::Abort) => break 'outer,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break 'outer,
                }
            }
            sleep(Duration::from_millis(interval));
            println!("Sending heartbeat");
            match socket_sender.send(Text("{\"op\": 1, \"d\": null}".to_string())).await {
                Ok(()) => (),
                Err(why) => println!("Error sending heartbeat: {:?}", why),
            };
        }
        println!("Gateway thread exiting");
    });
}

// async fn connect_gateway(token: String) {
//     let url = format!("wss://gateway.discord.gg/?v=9&encoding=json");
//     let (mut ws_stream, _) = connect_async(url)
//         .await
//         .expect("Failed to connect to Discord gateway");
//     println!("Connected to Discord gateway");
//     // Receive hello
//     let hello = ws_stream.next().await.unwrap().unwrap();
//     if let Text(hello) = hello {

//     } else {
//         panic!("Expected hello message");
//     }
//     // Identify with server
//     let identify = serde_json::json!({
//         "op": 2,
//         "d": {
//             "token": token,
//             "properties": {
//                 "$os": "windows",
//                 "$browser": "discord-rs",
//                 "$device": "discord-rs",
//             },
//         }
//     });
//     ws_stream.send(Text(serde_json::to_string(&identify).unwrap())).await.unwrap();

// }
