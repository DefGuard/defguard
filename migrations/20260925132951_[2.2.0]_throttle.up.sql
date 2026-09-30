CREATE TYPE throttle_scope AS ENUM (
    'web_login',
    'vpn_mfa_code',
    'vpn_mfa_initiate'
);
CREATE TABLE throttle (
    scope      throttle_scope NOT NULL,
    key        text NOT NULL,
    attempts   integer NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    PRIMARY KEY (scope, key)
);
