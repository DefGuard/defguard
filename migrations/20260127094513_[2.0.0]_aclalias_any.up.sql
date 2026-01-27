ALTER TABLE aclalias
  ADD COLUMN any_destination boolean NOT NULL DEFAULT true,
  ADD COLUMN any_port boolean NOT NULL DEFAULT true,
  ADD COLUMN any_protocol boolean NOT NULL DEFAULT true;
