UPDATE activity_log_event
SET ip = '0.0.0.0'::inet
WHERE ip IS NULL;

ALTER TABLE activity_log_event ALTER COLUMN ip SET NOT NULL;
