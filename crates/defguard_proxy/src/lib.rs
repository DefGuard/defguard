/// TODO(jck) rustdoc, list orchestrator's responsibilities
pub struct ProxyOrchestrator {
    pool: PgPool,
    proxies: Vec<Proxy>,
    tx: ProxyTxSet,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
}

impl ProxyOrchestrator {
    pub fn new(
        pool: PgPool,
        tx: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    ) -> Self {
        Self {
            pool,
            proxies: Vec::new(),
            tx,
            incompatible_components,
        }
    }

    /// TODO(jck) Retrieves proxies from the db and runs them
    // TODO(jck) consider new error type
    pub async fn run(mut self) -> Result<(), anyhow::Error> {
        // TODO(jck) retrieve proxies from db
        let mut proxies = vec![
            Proxy::new(
                self.pool.clone(),
                Uri::from_static("http://localhost:50051"),
                self.tx.clone(),
            )?,
            Proxy::new(
                self.pool.clone(),
                Uri::from_static("http://localhost:50052"),
                self.tx.clone(),
            )?,
        ];
        self.proxies.append(&mut proxies);
        let mut tasks = JoinSet::<Result<(), anyhow::Error>>::new();
        for proxy in self.proxies {
            tasks.spawn(proxy.run(self.tx.clone(), self.incompatible_components.clone()));
        }
        // TODO(jck) handle empty proxies vec somewhere earlier
        while let Some(result) = tasks.join_next().await {
            match result {
                // TODO(jck) add proxy id/name to the error log
                Ok(Ok(())) => error!("Proxy task returned prematurely"),
                Ok(Err(err)) => error!("Proxy task returned with error: {err}"),
                Err(err) => error!("Proxy task execution failed: {err}"),
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct ProxyTxSet {
    wireguard: Sender<GatewayEvent>,
    mail: UnboundedSender<Mail>,
    bidi_events: UnboundedSender<BidiStreamEvent>,
}

impl ProxyTxSet {
    pub fn new(
        wireguard: Sender<GatewayEvent>,
        mail: UnboundedSender<Mail>,
        bidi_events: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            wireguard,
            mail,
            bidi_events,
        }
    }
}

/// Groups all proxy GRPC servers
struct Proxy {
    pool: PgPool,
    /// Proxy server gRPC URI
    endpoint: Endpoint,
    /// gRPC servers
    servers: ProxyServerSet,
}

impl Proxy {
    // TODO(jck) better error
    pub fn new(pool: PgPool, uri: Uri, tx: ProxyTxSet) -> Result<Self, anyhow::Error> {
        let endpoint = Endpoint::from(uri);

        // Set endpoint keep-alive to avoid connectivity issues in proxied deployments.
        let endpoint = endpoint
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        // Setup certs.
        let config = server_config();
        let endpoint = if let Some(ca) = &config.proxy_grpc_ca {
            let ca = read_to_string(ca)?;
            let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
            endpoint.tls_config(tls)?
        } else {
            endpoint.tls_config(ClientTlsConfig::new().with_enabled_roots())?
        };

        // Instantiate gRPC servers.
        let servers = ProxyServerSet::new(pool.clone(), tx);

        Ok(Self {
            pool,
            endpoint,
            servers,
        })
    }
	// TODO(jck) fn run(), fn message_loop()
}

struct ProxyServerSet {
    enrollment: EnrollmentServer,
    password_reset: PasswordResetServer,
    client_mfa: ClientMfaServer,
    polling: PollingServer,
}

impl ProxyServerSet {
    pub fn new(pool: PgPool, tx: ProxyTxSet) -> Self {
        let enrollment = EnrollmentServer::new(
            pool.clone(),
            tx.wireguard.clone(),
            tx.mail.clone(),
            tx.bidi_events.clone(),
        );
        let password_reset =
            PasswordResetServer::new(pool.clone(), tx.mail.clone(), tx.bidi_events.clone());
        let client_mfa = ClientMfaServer::new(
            pool.clone(),
            tx.mail.clone(),
            tx.wireguard.clone(),
            tx.bidi_events.clone(),
        );
        let polling = PollingServer::new(pool.clone());

        Self {
            enrollment,
            password_reset,
            client_mfa,
            polling,
        }
    }
}
