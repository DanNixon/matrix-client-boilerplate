use anyhow::Result;
use matrix_sdk::{config::SyncSettings, ruma::UserId, Session};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

#[derive(Default)]
struct Inner {
    initial_sync_token: Option<String>,
    _sync_task: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct Client {
    client: matrix_sdk::Client,
    inner: Arc<Mutex<Inner>>,
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
            let session_file = std::fs::File::open(session_filename.clone())?;
            let session: Session = serde_json::from_reader(session_file)?;

            // Login
            client
                .login_username(user.localpart(), password)
                .initial_device_display_name(device_name)
                .device_id(session.device_id.as_str())
                .send()
                .await?;

            // Restore session
            // TODO: why does this not work correctly?
            // client.restore_login(session).await?;
        } else {
            log::info!("Initial login");

            // Login
            client
                .login_username(user.localpart(), password)
                .initial_device_display_name(device_name)
                .send()
                .await?;
        }

        // Save the session to file
        let session = client.session().expect("Session should exist after login");
        let session_file = std::fs::File::create(session_filename)?;
        serde_json::to_writer(session_file, &session)?;

        Ok(Self {
            client,
            inner: Default::default(),
        })
    }

    pub fn client(&self) -> &matrix_sdk::Client {
        &self.client
    }

    pub async fn initial_sync(&self) -> Result<()> {
        let response = self.client.sync_once(SyncSettings::default()).await?;
        self.inner.lock().unwrap().initial_sync_token = Some(response.next_batch);
        Ok(())
    }

    pub async fn start_background_sync(&self) {
        let client = self.client.clone();

        let mut inner = self.inner.lock().unwrap();

        let mut settings = SyncSettings::default();
        if let Some(token) = &inner.initial_sync_token {
            settings = settings.token(token);
        }

        inner._sync_task = Some(tokio::spawn(async move {
            client.sync(settings).await.unwrap();
        }));
    }
}
