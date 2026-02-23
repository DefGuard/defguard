ALTER TABLE gateway
    DROP COLUMN url,
    DROP COLUMN hostname,
    -- FIXME: remove the default once we squash alpha migrations
    ADD COLUMN address text NOT NULL DEFAULT '127.0.0.1',
    -- FIXME: remove the default once we squash alpha migrations
    ADD COLUMN port integer NOT NULL DEFAULT 50051,
    ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- FIXME: remove the default once we squash alpha migrations
    ADD COLUMN modified_by bigint NOT NULL DEFAULT 1,
    ADD CONSTRAINT proxy_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES "user"(id);

ALTER TABLE gateway RENAME COLUMN network_id TO location_id;
