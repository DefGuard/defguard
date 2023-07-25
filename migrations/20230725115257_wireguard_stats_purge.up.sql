CREATE TYPE stats_purge_trigger AS ENUM ('periodic_task', 'cli');

CREATE TABLE wireguard_stats_purge (
    id bigserial PRIMARY KEY,
    started_at timestamp without time zone NOT NULL,
    finished_at timestamp without time zone NOT NULL,
    removal_threshold timestamp without time zone NOT NULL,
    records_removed bigint NOT NULL,
    triggered_by stats_purge_trigger
);
