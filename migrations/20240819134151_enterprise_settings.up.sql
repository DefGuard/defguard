CREATE TABLE enterprisesettings (
    id bigserial PRIMARY KEY,
    disable_device_management BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO enterprisesettings (disable_device_management) values (false);
