-- add new toggle columns
ALTER TABLE aclalias
  ADD COLUMN any_address boolean NOT NULL DEFAULT false,
  ADD COLUMN any_port boolean NOT NULL DEFAULT false,
  ADD COLUMN any_protocol boolean NOT NULL DEFAULT false;

-- set values for new columns based on existing data
UPDATE aclalias SET
  any_address = array_length(destination, 1) IS NULL,
  any_port = array_length(ports, 1) IS NULL,
  any_protocol = array_length(protocols, 1) IS NULL;

-- rename destination column to avoid confusion
ALTER TABLE aclalias RENAME COLUMN destination TO addresses;

-- do the same for the aclrule table itself
ALTER TABLE aclrule
  ADD COLUMN any_address boolean NOT NULL DEFAULT false,
  ADD COLUMN any_port boolean NOT NULL DEFAULT false,
  ADD COLUMN any_protocol boolean NOT NULL DEFAULT false,
  ADD COLUMN use_manual_destination_settings boolean NOT NULL DEFAULT true,
  ADD COLUMN allow_all_groups boolean NOT NULL DEFAULT false,
  ADD COLUMN deny_all_groups boolean NOT NULL DEFAULT false;

UPDATE aclrule SET
  any_address = array_length(destination, 1) IS NULL,
  any_port = array_length(ports, 1) IS NULL,
  any_protocol = array_length(protocols, 1) IS NULL;

ALTER TABLE aclrule RENAME COLUMN destination TO addresses;
ALTER TABLE aclrule RENAME COLUMN all_networks TO all_locations;
