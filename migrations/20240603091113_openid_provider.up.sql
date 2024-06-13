CREATE TABLE openidprovider (
    id bigserial PRIMARY KEY,
    "name" text NOT NULL,
    "document_url" text NOT NULL,
    CONSTRAINT openidprovider_name_unique UNIQUE ("name")
);
