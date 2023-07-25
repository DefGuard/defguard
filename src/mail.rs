use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug, Clone)]
pub struct Mail {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub content: String,
}

pub struct MailHandler {
    rx: UnboundedReceiver<Mail>,
}

impl MailHandler {
    pub fn new(rx: UnboundedReceiver<Mail>) -> Self {
        Self { rx }
    }

    pub async fn run(mut self) {
        while let Some(mail) = self.rx.recv().await {
            debug!("Sending mail: {mail:?}");
            // TODO
            info!("Sent mail: {mail:?}");
        }
    }
}
