ALTER TABLE proxy
    ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN modified_by bigint NOT NULL DEFAULT 1,
    ADD CONSTRAINT proxy_modified_by_fkey
        FOREIGN KEY (modified_by) REFERENCES "user"(id);
