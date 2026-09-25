CREATE TABLE throttle (
    scope      text NOT NULL,
    key        text NOT NULL,
    attempts   integer NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    PRIMARY KEY (scope, key)
);
