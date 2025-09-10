ALTER TABLE token DROP CONSTRAINT enrollment_admin_id_fkey;
ALTER TABLE token ADD CONSTRAINT enrollment_admin_id_fkey FOREIGN KEY(admin_id) REFERENCES "user"(id);
