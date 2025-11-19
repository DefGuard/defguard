-- restore boolean `mfa_enabled` column
ALTER TABLE enterprisesettings ADD COLUMN "disable_all_traffic" BOOLEAN NOT NULL DEFAULT false;

-- populate based on client traffic policy
UPDATE enterprisesettings
SET disable_all_traffic = CASE
    WHEN client_traffic_policy = 'disable_all_traffic'::client_traffic_policy THEN true
    ELSE false
END;

-- drop new column and type
ALTER TABLE enterprisesettings DROP COLUMN "client_traffic_policy";
DROP TYPE client_traffic_policy;
