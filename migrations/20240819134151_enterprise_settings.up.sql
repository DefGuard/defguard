CREATE TABLE enterprisesettings (
    id bigserial PRIMARY KEY,
    admin_device_management BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO enterprisesettings (admin_device_management) values (false);
