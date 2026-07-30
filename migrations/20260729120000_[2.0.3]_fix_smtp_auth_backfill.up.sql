-- Repair rows mis-classified by the [2.0.2] smtp_xoauth2 back-fill.
-- That migration set smtp_authentication = 'login' whenever smtp_user and
-- smtp_password were merely NOT NULL, but empty-string credentials (valid and
-- common under 2.0.1, where SMTP authentication was optional) then fail the
-- stricter SmtpSettings::is_configured() check. That in turn makes settings
-- validation reject "gateway disconnect notifications" at startup and aborts
-- the core process. Fall back to 'none' for those rows, restoring 2.0.1 behavior.
UPDATE settings SET smtp_authentication = 'none'
  WHERE smtp_authentication = 'login'
    AND (smtp_user IS NULL OR smtp_user = ''
      OR smtp_password IS NULL OR smtp_password = '');
