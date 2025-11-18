-- add enum representing client traffic policy
CREATE TYPE client_traffic_policy AS ENUM (
    'none',
    'disable_all_traffic',
    'force_all_traffic'
);

-- add nullable column to `enterprisesettings` table
ALTER TABLE enterprisesettings ADD COLUMN "client_traffic_policy" client_traffic_policy DEFAULT 'none';

-- populate new column based on value in `disable_all_traffic` column
UPDATE enterprisesettings
SET client_traffic_policy = CASE
    WHEN disable_all_traffic = true THEN 'disable_all_traffic'::client_traffic_policy
    ELSE 'none'::client_traffic_policy
END;

-- make new column NOT NULL
ALTER TABLE enterprisesettings ALTER COLUMN "client_traffic_policy" SET NOT NULL;

-- drop the `disable_all_traffic` column since it's no longer needed
ALTER TABLE enterprisesettings DROP COLUMN "disable_all_traffic";
