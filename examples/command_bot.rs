use clap::Parser;
use matrix_sdk::{
    room::Room,
    ruma::events::room::message::{
        MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
    },
};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    /// Matrix username
    #[clap(value_parser, long)]
    pub(crate) username: String,

    /// Matrix password
    #[clap(value_parser, long)]
    pub(crate) password: String,

    /// Matrix storage directory
    #[clap(value_parser, long)]
    pub(crate) storage: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let matrix_client = matrix_client_boilerplate::Client::new(
        &args.username,
        &args.password,
        "command_bot",
        &args.storage,
    )
    .await
    .unwrap();

    matrix_client.initial_sync().await.unwrap();

    matrix_client.client().add_event_handler(on_room_message);

    matrix_client.start_background_sync().await;

    tokio::signal::ctrl_c().await.unwrap();
}

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) {
    if let Room::Joined(room) = room {
        let MessageType::Text(text_content) = event.content.msgtype else {
            return;
        };

        if text_content.body.contains("!party") {
            let content = RoomMessageEventContent::text_plain("🎉🎊🥳 let's PARTY!! 🥳🎊🎉");
            room.send(content, None).await.unwrap();
        }

        room.read_receipt(&event.event_id).await.unwrap();
    }
}
