ALTER TABLE device_posture_os_rule DROP COLUMN min_os_version;
ALTER TABLE device_posture_os_rule ADD COLUMN min_os_version integer;

ALTER TABLE device_posture_os_rule DROP COLUMN min_kernel_version;
ALTER TABLE device_posture_os_rule ADD COLUMN min_kernel_version integer;
