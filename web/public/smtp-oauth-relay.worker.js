// SharedWorker relay for SMTP OAuth code delivery.
// Relays messages between the parent window and the OAuth callback popup
// when COOP has severed window.opener (e.g. accounts.google.com sets
// Cross-Origin-Opener-Policy: same-origin). SharedWorkers cross browsing-context-
// group boundaries, unlike postMessage and BroadcastChannel.
const ports = new Set();

self.addEventListener('connect', (e) => {
  const port = e.ports[0];
  ports.add(port);

  port.addEventListener('message', (msg) => {
    for (const p of ports) {
      if (p !== port) p.postMessage(msg.data);
    }
  });

  port.start();
});
