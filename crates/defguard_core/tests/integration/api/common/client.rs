use std::{net::SocketAddr, sync::Arc};

use axum::{Router, serve};
use bytes::Bytes;
use defguard_common::db::Id;
use defguard_core::{
    events::{ApiEvent, ApiEventType},
    handlers::Auth,
};
use reqwest::{
    Body, Client, StatusCode, Url,
    cookie::{Cookie, Jar},
    header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT},
    redirect::Policy,
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{UnboundedReceiver, error::TryRecvError},
    task::JoinHandle,
};

pub struct TestClient {
    client: Client,
    jar: Arc<Jar>,
    port: u16,
    api_event_rx: UnboundedReceiver<ApiEvent>,
    // Has to live during whole test
    api_task_handle: JoinHandle<()>,
}

impl TestClient {
    #[must_use]
    pub fn new(
        app: Router,
        listener: TcpListener,
        api_event_rx: UnboundedReceiver<ApiEvent>,
    ) -> Self {
        let port = listener.local_addr().unwrap().port();

        let api_task_handle = tokio::spawn(async move {
            let server = serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            );
            server.await.expect("server error");
        });

        let jar = Arc::new(Jar::default());

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));

        let client = Client::builder()
            .default_headers(headers)
            .redirect(Policy::none())
            .cookie_provider(jar.clone())
            .build()
            .unwrap();

        TestClient {
            client,
            jar,
            port,
            api_event_rx,
            api_task_handle,
        }
    }

    pub fn set_cookie(&mut self, cookie: &Cookie) {
        let url = Url::parse(&self.base_url()).unwrap();
        self.jar
            .add_cookie_str(&format!("{}={}", cookie.name(), cookie.value()), &url);
    }

    // Helper to perform API login
    pub async fn login_user(&mut self, username: &str, password: &str) {
        let auth = Auth::new(username, password);
        let response = self.post("/api/v1/auth").json(&auth).send().await;
        assert_eq!(response.status(), StatusCode::OK);

        self.verify_api_events(&[ApiEventType::UserLogin]);
    }

    /// returns the base URL (http://ip:port) for this TestClient
    ///
    /// this is useful when trying to check if Location headers in responses
    /// are generated correctly as Location contains an absolute URL
    pub fn base_url(&self) -> String {
        let mut s = String::from("http://localhost:");
        s.push_str(&self.port.to_string());
        s
    }

    pub fn get<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.get(full_url),
        }
    }

    pub fn head<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.head(full_url),
        }
    }

    pub fn post<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.post(full_url),
        }
    }

    pub fn put<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.put(full_url),
        }
    }

    pub fn patch<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.patch(full_url),
        }
    }

    pub fn delete<T: AsRef<str>>(&self, url: T) -> RequestBuilder {
        let mut full_url = self.base_url();
        full_url.push_str(url.as_ref());
        RequestBuilder {
            builder: self.client.delete(full_url),
        }
    }

    /// Assert that expected API events have been emitted
    ///
    /// `expected_events` should include all events that are currently in the queue
    /// for the assertions to pass.
    /// If there are too many or not enough events in the queue this should panic.
    pub fn verify_api_events(&mut self, expected_events: &[ApiEventType]) {
        // take all the events from the queue
        let events = self.drain_all_events();

        // verify number of events
        assert_eq!(
            events.len(),
            expected_events.len(),
            "Event number different than expected"
        );

        // compare events in order
        for (index, (expected_event, (event, _user_id, _username))) in
            expected_events.iter().zip(events.iter()).enumerate()
        {
            assert_eq!(
                expected_event, event,
                "Mismatch at index {}: expected {:?}, got {:?}",
                index, expected_event, event
            );
        }
    }

    /// A variant of `verify_api_events` which also compares user context
    ///
    /// Other parts of event context that would be hard and not that useful to test (timestamp, device) are omitted.
    pub fn verify_api_events_with_user(&mut self, expected_events: &[(ApiEventType, Id, &str)]) {
        // take all the events from the queue
        let events = self.drain_all_events();

        // verify number of events
        assert_eq!(
            events.len(),
            expected_events.len(),
            "Event number different than expected"
        );

        // compare events in order
        for (
            index,
            ((expected_event, expected_user_id, expected_username), (event, user_id, username)),
        ) in expected_events.iter().zip(events.iter()).enumerate()
        {
            assert_eq!(
                expected_event, event,
                "Event type mismatch at index {}: expected {:#?}, got {:#?}",
                index, expected_event, event
            );
            assert_eq!(
                expected_user_id, user_id,
                "User ID mismatch at index {}: expected {:?}, got {:?}",
                index, expected_user_id, user_id
            );
            assert_eq!(
                expected_username, username,
                "Username mismatch at index {}: expected {:?}, got {:?}",
                index, expected_username, username
            );
        }
    }

    /// Receive all messages currently present in API event queue
    ///
    /// Can also be used to clear the queue.
    pub fn drain_all_events(&mut self) -> Vec<(ApiEventType, Id, String)> {
        let mut all_events = Vec::new();

        loop {
            match self.api_event_rx.try_recv() {
                Ok(msg) => all_events.push((*msg.event, msg.context.user_id, msg.context.username)),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No more messages available right now
                    break;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Channel is closed
                    break;
                }
            }
        }
        all_events
    }

    /// Assert there are no events queued
    pub fn assert_event_queue_is_empty(&mut self) {
        match self.api_event_rx.try_recv() {
            Err(TryRecvError::Empty) => {
                // Queue is empty, test passes
            }
            Ok(msg) => panic!("Expected empty queue, but got event: {:?}", msg),
            Err(TryRecvError::Disconnected) => panic!("Channel is disconnected"),
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // explicitly stop spawned API server task
        self.api_task_handle.abort();
    }
}

pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    pub async fn send(self) -> TestResponse {
        TestResponse {
            response: self.builder.send().await.unwrap(),
        }
    }

    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    // pub fn form<T: serde::Serialize + ?Sized>(mut self, form: &T) -> Self {
    //     self.builder = self.builder.form(&form);
    //     self
    // }

    pub fn json<T>(mut self, json: &T) -> Self
    where
        T: serde::Serialize,
    {
        self.builder = self.builder.json(json);
        self
    }

    pub fn header(mut self, key: HeaderName, value: &str) -> Self {
        self.builder = self.builder.header(key, value);
        self
    }

    // pub fn multipart(mut self, form: reqwest::multipart::Form) -> Self {
    //     self.builder = self.builder.multipart(form);
    //     self
    // }
}

/// A wrapper around [`reqwest::Response`] that provides common methods with internal `unwrap()`s.
///
/// This is conventient for tests where panics are what you want. For access to
/// non-panicking versions or the complete `Response` API use `into_inner()` or
/// `as_ref()`.
pub struct TestResponse {
    response: reqwest::Response,
}

impl TestResponse {
    pub async fn text(self) -> String {
        self.response.text().await.unwrap()
    }

    pub async fn bytes(self) -> Bytes {
        self.response.bytes().await.unwrap()
    }

    pub async fn json<T>(self) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        self.response.json().await.unwrap()
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub fn cookies(&self) -> impl Iterator<Item = Cookie<'_>> {
        self.response.cookies()
    }

    // pub async fn chunk(&mut self) -> Option<Bytes> {
    //     self.response.chunk().await.unwrap()
    // }

    // pub async fn chunk_text(&mut self) -> Option<String> {
    //     let chunk = self.chunk().await?;
    //     Some(String::from_utf8(chunk.to_vec()).unwrap())
    // }

    /// Get the inner [`reqwest::Response`] for less convenient but more complete access.
    pub fn into_inner(self) -> reqwest::Response {
        self.response
    }
}

impl AsRef<reqwest::Response> for TestResponse {
    fn as_ref(&self) -> &reqwest::Response {
        &self.response
    }
}
