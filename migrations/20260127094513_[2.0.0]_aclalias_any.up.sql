ALTER TABLE aclalias
  ADD COLUMN any_destination boolean NOT NULL DEFAULT false,
  ADD COLUMN any_port boolean NOT NULL DEFAULT false,
  ADD COLUMN any_protocol boolean NOT NULL DEFAULT false;
UPDATE aclalias SET
  any_destination = array_length(destination, 1) IS NULL,
  any_port = array_length(ports, 1) IS NULL,
  any_protocol = array_length(protocols, 1) IS NULL;
ALTER TABLE aclrule
  ADD COLUMN any_destination boolean NOT NULL DEFAULT false,
  ADD COLUMN any_port boolean NOT NULL DEFAULT false,
  ADD COLUMN any_protocol boolean NOT NULL DEFAULT false,
  ADD COLUMN manual_settings boolean NOT NULL DEFAULT false;
UPDATE aclrule SET
  any_destination = array_length(destination, 1) IS NULL,
  any_port = array_length(ports, 1) IS NULL,
  any_protocol = array_length(protocols, 1) IS NULL;
