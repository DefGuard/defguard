ALTER TABLE settings ADD gateway_disconnect_notifications_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD gateway_disconnect_notifications_inactivity_threshold INT4 NOT NULL DEFAULT 5;
ALTER TABLE settings ADD gateway_disconnect_notifications_reconnect_notification_enabled BOOLEAN NOT NULL DEFAULT FALSE;
