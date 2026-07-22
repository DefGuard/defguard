//! In-process mock SMTP server for integration tests.
//!
//! [`MockSmtpServer`] speaks just enough of the SMTP dialogue (plaintext, no
//! authentication) for lettre's `AsyncSmtpTransport` to deliver a message,
//! captures every delivered message, and can point the current settings at
//! itself so that `Mail::send()` reaches it.
//!
//! Because most mail is sent fire-and-forget (`tokio::spawn`) the delivery
//! happens after the triggering request returns, so tests should wait for a
//! captured message with [`MockSmtpServer::wait_for`] /
//! [`MockSmtpServer::wait_for_count`] rather than reading synchronously.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use sqlx::PgPool;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};
use tracing::debug;

use crate::db::models::settings::{
    Settings,
    smtp::{SmtpAuthentication, SmtpEncryption},
    update_current_settings,
};

/// Default time [`MockSmtpServer::wait_for`] and friends will poll before
/// giving up.
pub const DEFAULT_MAIL_TIMEOUT: Duration = Duration::from_secs(5);

/// A single message delivered to a [`MockSmtpServer`].
#[derive(Debug, Clone)]
pub struct CapturedMail {
    /// Address from the `MAIL FROM` command.
    pub from: String,
    /// Addresses from the `RCPT TO` commands.
    pub recipients: Vec<String>,
    /// Raw payload sent after `DATA` (MIME headers + body), with SMTP
    /// dot-unstuffing applied and the trailing `.` removed.
    pub body: String,
}

impl CapturedMail {
    /// Whether `recipients` contains `address`.
    #[must_use]
    pub fn sent_to(&self, address: &str) -> bool {
        self.recipients.iter().any(|r| r == address)
    }

    /// Whether the raw payload contains `needle`.
    #[must_use]
    pub fn body_contains(&self, needle: &str) -> bool {
        self.body.contains(needle)
    }
}

/// In-process SMTP server that accepts any message and records it.
///
/// The listener is owned by a background task, so the server keeps running even
/// if this handle is dropped; it stops when the test's tokio runtime shuts
/// down.
pub struct MockSmtpServer {
    addr: SocketAddr,
    received: Arc<Mutex<Vec<CapturedMail>>>,
}

impl MockSmtpServer {
    /// Bind an ephemeral port on localhost and start accepting connections.
    #[must_use = "the returned handle exposes the captured messages"]
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock SMTP listener");
        let addr = listener
            .local_addr()
            .expect("failed to read mock SMTP local address");
        debug!("Mock SMTP server listening on {addr}");

        let received = Arc::new(Mutex::new(Vec::new()));
        let received_bg = Arc::clone(&received);
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let received_conn = Arc::clone(&received_bg);
                tokio::spawn(handle_connection(stream, received_conn));
            }
        });

        Self { addr, received }
    }

    /// Address the server is listening on.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Point the current settings (global cache + database) at this server,
    /// using plaintext transport with no authentication.
    pub async fn configure(&self, pool: &PgPool) {
        let mut settings = Settings::get_current_settings();
        settings.smtp.server = Some(self.addr.ip().to_string());
        settings.smtp.port = Some(i32::from(self.addr.port()));
        settings.smtp.sender = Some("noreply@example.com".into());
        settings.smtp.encryption = SmtpEncryption::None;
        settings.smtp.authentication = SmtpAuthentication::None;
        update_current_settings(pool, settings)
            .await
            .expect("failed to persist mock SMTP settings");
    }

    /// Snapshot of all messages received so far.
    #[must_use]
    pub fn messages(&self) -> Vec<CapturedMail> {
        self.received.lock().unwrap().clone()
    }

    /// Number of messages received so far.
    #[must_use]
    pub fn message_count(&self) -> usize {
        self.received.lock().unwrap().len()
    }

    /// Wait until at least `n` messages have been received, then return a
    /// snapshot of all captured messages.
    ///
    /// # Panics
    /// Panics if [`DEFAULT_MAIL_TIMEOUT`] elapses first.
    pub async fn wait_for_count(&self, n: usize) -> Vec<CapturedMail> {
        let poll = async {
            loop {
                {
                    let guard = self.received.lock().unwrap();
                    if guard.len() >= n {
                        return guard.clone();
                    }
                }
                sleep(Duration::from_millis(20)).await;
            }
        };
        timeout(DEFAULT_MAIL_TIMEOUT, poll)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "timed out waiting for {n} mail(s); received {}",
                    self.message_count()
                )
            })
    }

    /// Wait for the first captured message matching `predicate` and return it.
    ///
    /// Searches the whole capture buffer on every poll, so it is robust to
    /// unrelated fire-and-forget mails (e.g. new-device-login notifications)
    /// arriving in between.
    ///
    /// # Panics
    /// Panics if [`DEFAULT_MAIL_TIMEOUT`] elapses before a match appears.
    pub async fn wait_for<F>(&self, predicate: F) -> CapturedMail
    where
        F: Fn(&CapturedMail) -> bool,
    {
        self.wait_for_from(0, predicate).await.1
    }

    /// Like [`wait_for`](Self::wait_for), but only considers messages at index
    /// `>= start` in arrival order. Returns the matched message together with
    /// its absolute index, so a caller stepping through a multi-mail flow can
    /// advance a cursor (`start = index + 1`) and ignore already-consumed mail.
    ///
    /// # Panics
    /// Panics if [`DEFAULT_MAIL_TIMEOUT`] elapses before a match appears.
    pub async fn wait_for_from<F>(&self, start: usize, predicate: F) -> (usize, CapturedMail)
    where
        F: Fn(&CapturedMail) -> bool,
    {
        let poll = async {
            loop {
                {
                    let guard = self.received.lock().unwrap();
                    if let Some((index, mail)) = guard
                        .iter()
                        .enumerate()
                        .skip(start)
                        .find(|(_, m)| predicate(m))
                    {
                        return (index, mail.clone());
                    }
                }
                sleep(Duration::from_millis(20)).await;
            }
        };
        timeout(DEFAULT_MAIL_TIMEOUT, poll)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for a matching mail"))
    }
}

/// Convenience wrapper: start a server and point the current settings at it.
///
/// Mirrors the previous per-crate helper of the same name; prefer constructing
/// a [`MockSmtpServer`] directly when you need to inspect captured mail.
pub async fn configure_working_smtp(pool: &PgPool) -> MockSmtpServer {
    let server = MockSmtpServer::start().await;
    server.configure(pool).await;
    server
}

/// Extract the `<...>` address from a `MAIL FROM` / `RCPT TO` command line.
fn extract_address(line: &str) -> String {
    match (line.find('<'), line.find('>')) {
        (Some(start), Some(end)) if start < end => line[start + 1..end].to_string(),
        // Fall back to whatever follows the ':' if the client omitted brackets.
        _ => line
            .split_once(':')
            .map_or_else(String::new, |(_, rest)| rest.trim().to_string()),
    }
}

/// Handle a single SMTP connection, recording any delivered message.
async fn handle_connection(stream: TcpStream, received: Arc<Mutex<Vec<CapturedMail>>>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    if writer.write_all(b"220 localhost ESMTP\r\n").await.is_err() {
        return;
    }

    let mut from = String::new();
    let mut recipients = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let upper = line.trim_end().to_ascii_uppercase();

        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            let _ = writer.write_all(b"250 localhost\r\n").await;
        } else if upper.starts_with("MAIL FROM") {
            from = extract_address(line.trim_end());
            let _ = writer.write_all(b"250 OK\r\n").await;
        } else if upper.starts_with("RCPT TO") {
            recipients.push(extract_address(line.trim_end()));
            let _ = writer.write_all(b"250 OK\r\n").await;
        } else if upper.starts_with("RSET") {
            from.clear();
            recipients.clear();
            let _ = writer.write_all(b"250 OK\r\n").await;
        } else if upper.starts_with("DATA") {
            let _ = writer
                .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                .await;
            let mut body = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                if line == ".\r\n" || line == ".\n" {
                    break;
                }
                // Undo SMTP dot-stuffing of lines that begin with '.'.
                let content = line.strip_prefix('.').unwrap_or(&line);
                body.push_str(content);
            }
            // Record the message before acknowledging, so that an awaited
            // `send()` observes the capture as soon as it returns.
            received.lock().unwrap().push(CapturedMail {
                from: std::mem::take(&mut from),
                recipients: std::mem::take(&mut recipients),
                body,
            });
            let _ = writer.write_all(b"250 OK message queued\r\n").await;
        } else if upper.starts_with("QUIT") {
            let _ = writer.write_all(b"221 Bye\r\n").await;
            return;
        } else {
            let _ = writer.write_all(b"250 OK\r\n").await;
        }
    }
}
