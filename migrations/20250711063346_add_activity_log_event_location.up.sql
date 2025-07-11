ALTER TABLE activity_log_event ADD COLUMN "location" TEXT;

CREATE INDEX activity_log_event_location_idx ON activity_log_event(location);
