use std::{net::SocketAddr, sync::Arc};

use axum::{serve, Router};
use bytes::Bytes;
use reqwest::{
    cookie::{Cookie, Jar},
    header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT},
    redirect::Policy,
    Body, Client, StatusCode, Url,
};
use tokio::net::TcpListener;

pub struct TestClient {
    client: Client,
    jar: Arc<Jar>,
    port: u16,
}

impl TestClient {
    #[must_use]
    pub fn new(app: Router, listener: TcpListener) -> Self {
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
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

        TestClient { client, jar, port }
    }

    pub fn set_cookie(&mut self, cookie: &Cookie) {
        let url = Url::parse(&self.base_url()).unwrap();
        self.jar
            .add_cookie_str(&format!("{}={}", cookie.name(), cookie.value()), &url);
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
#[derive(Debug)]

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
