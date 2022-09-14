pub mod discord;

use async_std::sync::Mutex;
use discord::gateway::GatewayClient;
use eframe::{
    epaint::ahash::{HashMap, HashMapExt},
    run_native, App,
};
use egui::ScrollArea;
use poll_promise::Promise;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

lazy_static::lazy_static! {
    // static ref GATEWAY_CLIENT: Arc<GatewayClient> = Arc::new(GatewayClient::new());
    static ref CHANNEL_CACHE: Arc<Mutex<HashMap<String, Vec<Channel>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref MESSAGE_CACHE: Arc<Mutex<HashMap<String, Vec<Message>>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Message {
    id: String,
    content: String,
    attachments: Vec<MessageAttachment>,
    author: MessageAuthor,
    channel_id: String,
    components: Vec<MessageComponent>,
    embeds: Vec<MessageEmbed>,
    edited_timestamp: Option<String>,
    flags: Option<u64>,
    mention_everyone: bool,
    mention_roles: Vec<String>,
    mentions: Vec<MessageMention>,
    pinned: bool,
    reactions: Option<Vec<MessageReaction>>,
    timestamp: String,
    tts: bool,
    #[serde(rename = "type")]
    message_type: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageAttachment {
    id: String,
    filename: String,
    content_type: Option<String>,
    size: u64,
    url: String,
    proxy_url: String,
    height: Option<u64>,
    width: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageAuthor {
    id: String,
    username: String,
    avatar: Option<String>,
    discriminator: String,
    public_flags: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageComponent {
    #[serde(rename = "type")]
    component_type: u64,
    style: Option<u64>,
    label: Option<String>,
    emoji: Option<MessageEmoji>,
    custom_id: Option<String>,
    url: Option<String>,
    disabled: Option<bool>,
    components: Option<Vec<MessageComponent>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageEmoji {
    id: Option<String>,
    name: Option<String>,
    animated: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageEmbed {
    title: Option<String>,
    #[serde(rename = "type")]
    embed_type: String,
    description: Option<String>,
    url: Option<String>,
    timestamp: Option<String>,
    color: Option<u64>,
    // footer: Option<MessageEmbedFooter>,
    // image: Option<MessageEmbedImage>,
    // thumbnail: Option<MessageEmbedThumbnail>,
    // video: Option<MessageEmbedVideo>,
    // provider: Option<MessageEmbedProvider>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageReaction {
    count: u64,
    me: bool,
    emoji: MessageEmoji,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MessageMention {
    id: String,
    username: String,
    discriminator: String,
    avatar: Option<String>,
    public_flags: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Guild {
    features: Vec<String>,
    icon: Option<String>,
    id: String,
    name: String,
    owner: bool,
    permissions: u64,
    permissions_new: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Channel {
    flags: u64,
    guild_id: Option<String>,
    id: String,
    name: String,
    nsfw: Option<bool>,
    parent_id: Option<String>,
    permission_overwrites: Vec<PermissionOverwrite>,
    position: u64,
    rate_limit_per_user: Option<u64>,
    topic: Option<String>,
    r#type: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PermissionOverwrite {
    allow: u64,
    deny: u64,
    id: String,
    r#type: String,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct DiscordClient {
    token: Option<String>,
    // #[serde(skip)]
    // client: Option<discord::Client>,
    #[serde(skip)]
    guilds: Option<Promise<Result<Vec<Guild>, String>>>,
    #[serde(skip)]
    gateway: Option<Promise<Result<GatewayClient, String>>>,
    #[serde(skip)]
    channels: Option<Promise<Result<Vec<Channel>, String>>>,
    #[serde(skip)]
    selected_guild: Option<String>,
    #[serde(skip)]
    selected_channel: Option<String>,
    #[serde(skip)]
    message: String,
    #[serde(skip)]
    messages: Option<Promise<Result<Vec<Message>, String>>>,
}

impl Default for DiscordClient {
    fn default() -> Self {
        Self {
            token: None,
            guilds: None,
            gateway: None,
            channels: None,
            selected_guild: None,
            selected_channel: None,
            message: String::new(),
            messages: None,
        }
    }
}

impl App for DiscordClient {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(token) = &mut self.token.clone() {
            // let (tx, rx) = self.thread.get_or_insert_with(|| {
            // let (tx, rx) = channel();
            // thread::spawn(move || {
            //     let client = discord::Client::new(token, discord::Discord::new(token).unwrap()).unwrap();
            //     let (mut connection, _) = client.start().unwrap();
            //     loop {
            //         match connection.recv_event() {
            //             Ok(discord::Event::MessageCreate(message)) => {
            //                 tx.send(message).unwrap();
            //             }
            //             Ok(_) => {}
            //             Err(discord::Error::Closed(code, body)) => {
            //                 println!("Gateway closed on us with code {:?}: {}", code, body);
            //                 break;
            //             }
            //             Err(err) => println!("Receive error: {:?}", err),
            //         }
            //     }
            // })

            // });

            self.gateway.get_or_insert_with(|| {
                let t = token.clone();
                Promise::spawn_async(async move {
                    let mut client = GatewayClient::new(t);
                    client.start().await;
                    Ok(client)
                })
            });

            self.guilds.get_or_insert_with(|| {
                let t = token.clone();
                Promise::spawn_async(async move {
                    let request = reqwest::Client::new()
                        .get("https://discord.com/api/users/@me/guilds")
                        .header("Authorization", t)
                        .send()
                        .await;
                    match request {
                        Ok(response) => {
                            let guilds = response.json::<Vec<Guild>>().await;
                            // let text = response.text().await.unwrap();
                            match guilds {
                                Ok(guilds) => Ok(guilds),
                                Err(err) => Err(format!("Failed to parse guilds: {}", err)),
                                // Err(_) => Err("Failed to parse guilds - is your token valid?".to_string()),
                            }
                            // Err(text)
                        }
                        Err(err) => Err(err.to_string()),
                    }
                })
            });

            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Log out").clicked() {
                            self.token = None;
                            self.guilds = None;
                            self.gateway = None;
                            self.channels = None;
                            self.selected_guild = None;
                            self.selected_channel = None;
                            self.message = String::new();
                            ctx.request_repaint();
                        }
                        if ui.button("Quit").clicked() {
                            frame.close();
                        }
                    });
                });
            });

            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                ui.heading("Welcome to Discord Client");

                ui.horizontal(|ui| {
                    // ui.label("Write something: ");
                    // ui.text_edit_singleline(&mut "a");
                    if let Some(gateway) = &self.gateway {
                        if let Some(gateway) = gateway.ready() {
                            if let Ok(gateway) = gateway {
                                ui.label("Connected to gateway");
                            } else if let Err(err) = gateway {
                                ui.label(err);
                            }
                        }
                    }
                });
                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(guilds) = &self.guilds {
                        if let Some(guilds) = guilds.ready() {
                            if let Ok(guilds) = guilds {
                                for guild in guilds {
                                    // ui.label(guild.name.clone());
                                    let label = ui.selectable_label(
                                        self.selected_guild.as_ref().unwrap_or(&"".to_string())
                                            == &guild.id,
                                        guild.name.clone(),
                                    );
                                    if label.clicked()
                                        && (self.selected_guild.as_ref().unwrap_or(&"".to_string())
                                            != &guild.id)
                                    {
                                        self.selected_guild = Some(guild.id.clone());
                                        let t = token.clone();
                                        let id = guild.id.clone();
                                        self.channels = Some(Promise::spawn_async(async move {
                                            let mut cache = CHANNEL_CACHE.lock().await;
                                            if let Some(channels) = cache.get(&id) {
                                                println!("{:?}", channels.clone());
                                                Ok(channels.clone())
                                            } else {
                                                let request = reqwest::Client::new()
                                                    .get(format!(
                                                        "https://discord.com/api/guilds/{}/channels",
                                                        &id
                                                    ))
                                                    .header("Authorization", t)
                                                    .send()
                                                    .await;
                                                match request {
                                                    Ok(response) => {
                                                        let channels =
                                                            response.json::<Vec<Channel>>().await;
                                                        match channels {
                                                            Ok(channels) => {
                                                                cache.insert(
                                                                    id.clone(),
                                                                    channels.clone(),
                                                                );
                                                                println!("{:?}", channels.clone());
                                                                Ok(channels)
                                                            }
                                                            Err(err) => Err(format!(
                                                                "Failed to parse channels: {}",
                                                                err
                                                            )),
                                                        }
                                                    }
                                                    Err(err) => Err(err.to_string()),
                                                }
                                            }
                                        }));
                                    }
                                }
                            } else if let Err(err) = guilds {
                                ui.label(err);
                            }
                        }
                    }
                });

                // ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                //     ui.horizontal(|ui| {
                //         ui.spacing_mut().item_spacing.x = 0.0;
                //         if let Some(gateway) = gateway_promise.ready() {
                //             if let Ok(gateway) = gateway {
                //                 ui.label("Connected to gateway");
                //             } else if let Err(err) = gateway {
                //                 ui.label(err);
                //             }
                //         }
                //     });
                // });
            });

            egui::SidePanel::left("side_panel_channels").show(ctx, |ui| {
                ui.heading("Channels");
                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(channels) = &self.channels {
                        if let Some(channels) = channels.ready() {
                            if let Ok(channels) = channels {
                                for channel in channels {
                                    let label = ui.selectable_label(
                                        self.selected_channel.as_ref().unwrap_or(&"".to_string())
                                            == &channel.id,
                                        channel.name.clone(),
                                    );
                                    if label.clicked() {
                                        self.selected_channel = Some(channel.id.clone());
                                        let t = token.clone();
                                        let id = channel.id.clone();
                                        self.messages = Some(Promise::spawn_async(async move {
                                            let mut cache = MESSAGE_CACHE.lock().await;
                                            if let Some(messages) = cache.get(&id) {
                                                println!("{:?}", messages.clone());
                                                Ok(messages.clone())
                                            } else {
                                                let request = reqwest::Client::new()
                                                    .get(format!(
                                                        "https://discord.com/api/channels/{}/messages",
                                                        &id
                                                    ))
                                                    .header("Authorization", t)
                                                    .send()
                                                    .await;
                                                match request {
                                                    Ok(response) => {
                                                        let messages =
                                                            response.json::<Vec<Message>>().await;
                                                        match messages {
                                                            Ok(messages) => {
                                                                cache.insert(
                                                                    id.clone(),
                                                                    messages.clone(),
                                                                );
                                                                println!("{:?}", messages.clone());
                                                                Ok(messages)
                                                            }
                                                            Err(err) => Err(format!(
                                                                "Failed to parse messages: {}",
                                                                err
                                                            )),
                                                        }
                                                    }
                                                    Err(err) => Err(err.to_string()),
                                                }
                                            }
                                        }));
                                    }
                                }
                            } else if let Err(err) = channels {
                                ui.label(err);
                            }
                        }
                    }
                });
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Discord Client");
                // ui.add(egui::TextEdit::singleline(token));
                // ui.add(egui::Button::new("Log in").on_hover_text("Log in to Discord"));
                if let Some(selected_channel) = &self.selected_channel {
                    ScrollArea::vertical().show(ui, |ui| {
                        if let Some(messages) = &self.messages {
                            if let Some(messages) = messages.ready() {
                                if let Ok(messages) = messages {
                                    for message in messages {
                                        ui.label(format!(
                                            "{}: {}",
                                            message.author.username, message.content
                                        ));
                                    }
                                } else if let Err(err) = messages {
                                    ui.label(err);
                                }
                            }
                        }
                    });
                    // Display text box for messages at the bottom of the panel
                    ui.add(egui::TextEdit::multiline(&mut self.message));
                    // Display button to send message
                    if ui
                        .add(egui::Button::new("Send message"))
                        .clicked()
                    {
                        let t = token.clone();
                        let s = selected_channel.clone();
                        let m = self.message.clone();
                        let _ = Promise::spawn_async(async move {
                            let request = reqwest::Client::new()
                                .post(format!("https://discord.com/api/v9/channels/{}/messages", s))
                                .header("Authorization", t)
                                .json(&serde_json::json!({
                                    // "content": "Hello world!",
                                    "content": m,
                                    "tts": false,
                                    "nonce": Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
                                }))
                                .send()
                                .await;
                            match request {
                                Ok(a) => {
                                    println!("{}", a.text().await.unwrap());
                                    Ok("success".to_string())
                                },
                                Err(err) => {
                                    println!("{}", err);
                                    Err(err.to_string())
                                },
                            }
                        });
                        self.message = String::new();
                    }
                }
            });
        } else {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            frame.close();
                        }
                    });
                });
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Discord Client");
                ui.label("Please enter your token");
                let mut token = String::new();
                let text = ui.add(egui::TextEdit::singleline(&mut token));
                if text.changed() {
                    self.token = Some(token);
                }
                if ui.button("Save").clicked() {
                    self.token = Some("".to_string());
                }
            });
        }
    }
}

impl DiscordClient {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

#[async_std::main]
async fn main() {
    let native_options = eframe::NativeOptions::default();
    run_native(
        "Discord Client",
        native_options,
        Box::new(|cc| Box::new(DiscordClient::new(cc))),
    );
}
