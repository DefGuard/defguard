ALTER TYPE mfa_method ADD VALUE 'web3';
CREATE TABLE wallet (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    address text NOT NULL UNIQUE,
    challenge_message text NOT NULL,
    challenge_signature text NULL,
    creation_timestamp timestamp without time zone NOT NULL,
    validation_timestamp timestamp without time zone NULL,
    name text NOT NULL DEFAULT '',
    chain_id bigint NOT NULL DEFAULT 0
);
ALTER TABLE session ADD web3_challenge text NULL;
