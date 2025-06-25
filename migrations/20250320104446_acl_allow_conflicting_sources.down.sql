ALTER TABLE aclruleuser DROP CONSTRAINT rule_user;
ALTER TABLE aclruleuser ADD CONSTRAINT rule_user UNIQUE (rule_id, user_id);

ALTER TABLE aclrulegroup DROP CONSTRAINT rule_group;
ALTER TABLE aclrulegroup ADD CONSTRAINT rule_group UNIQUE (rule_id, group_id);

ALTER TABLE aclruledevice DROP CONSTRAINT rule_device;
ALTER TABLE aclruledevice ADD CONSTRAINT rule_device UNIQUE (rule_id, device_id);
