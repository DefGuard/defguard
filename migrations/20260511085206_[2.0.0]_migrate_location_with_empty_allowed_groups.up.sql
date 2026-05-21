-- In 1.6.x an empty allowed groups list meant all groups have access to location.
-- Restore that meaning for locations that were migrated without any explicit group assigned.
UPDATE wireguard_network AS location
SET allow_all_groups = true
WHERE NOT EXISTS (
    SELECT 1
    FROM wireguard_network_allowed_group AS allowed_group
    WHERE allowed_group.network_id = location.id
);
