ALTER TABLE aclrule
  DROP COLUMN any_address,
  DROP COLUMN any_port,
  DROP COLUMN any_protocol,
  DROP COLUMN use_manual_destination_settings,
  DROP COLUMN allow_all_groups,
  DROP COLUMN deny_all_groups;
ALTER TABLE aclrule RENAME COLUMN addresses TO destination;
ALTER TABLE aclrule RENAME COLUMN all_locations TO all_networks;

ALTER TABLE aclalias
  DROP COLUMN any_address,
  DROP COLUMN any_port,
  DROP COLUMN any_protocol;
ALTER TABLE aclalias RENAME COLUMN addresses TO destination;
