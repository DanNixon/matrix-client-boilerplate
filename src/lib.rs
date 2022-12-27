use anyhow::Result;
use matrix_sdk::{config::SyncSettings, ruma::UserId};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Client {
    client: matrix_sdk::Client,
    _sync_task: Arc<Mutex<JoinHandle<()>>>,
}

impl Client {
    pub async fn new(
        username: &str,
        password: &str,
        device_name: &str,
        storage: &Path,
    ) -> Result<Self> {
        std::fs::create_dir_all(storage)?;

        let user = UserId::parse(username)?;

        let client = matrix_sdk::Client::builder()
            .server_name(user.server_name())
            .sled_store(storage.join("sled"), None)?
            .build()
            .await?;

        let session_filename = storage.join("session.json");
        if session_filename.exists() {
            log::info!("Restored login");

            // Load the session from file
            let session_file = std::fs::File::open(session_filename)?;
            let session = serde_json::from_reader(session_file)?;

            // Login
            client.restore_login(session).await?;
        } else {
            log::info!("Initial login");

            // Login
            client
                .login_username(user.localpart(), password)
                .initial_device_display_name(device_name)
                .send()
                .await?;

            // Save the session to file
            let session = client.session().expect("Session should exist after login");
            let session_file = std::fs::File::create(session_filename)?;
            serde_json::to_writer(session_file, &session)?;
        }

        log::info!("Performing initial sync");
        client.sync_once(SyncSettings::default()).await?;

        log::info!("Successfully logged in to Matrix homeserver");

        let sync_task = Arc::new(Mutex::new(sync_background(client.clone()).await));

        Ok(Self {
            client,
            _sync_task: sync_task,
        })
    }

    pub fn client(&self) -> &matrix_sdk::Client {
        &self.client
    }
}

pub(crate) async fn sync_background(client: matrix_sdk::Client) -> JoinHandle<()> {
    tokio::spawn(async move {
        let settings = SyncSettings::default().token(
            client
                .sync_token()
                .await
                .expect("sync token should be available"),
        );

        client.sync(settings).await.unwrap();
    })
}
