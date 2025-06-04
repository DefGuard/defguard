export type AuditModule = 'defguard' | 'client' | 'vpn' | 'enrollment';

export const auditModuleValues: AuditModule[] = [
  'defguard',
  'client',
  'enrollment',
  'vpn',
];

export type AuditEventType =
  | 'user_login'
  | 'user_logout'
  | 'user_added'
  | 'user_modified'
  | 'user_removed'
  | 'mfa_disabled'
  | 'mfa_totp_enabled'
  | 'mfa_totp_disabled'
  | 'mfa_email_enabled'
  | 'mfa_email_disabled'
  | 'mfa_security_key_added'
  | 'mfa_security_key_removed'
  | 'device_added'
  | 'device_modified'
  | 'device_removed'
  | 'network_device_added'
  | 'network_device_modified'
  | 'network_device_removed'
  | 'audit_stream_created'
  | 'audit_stream_modified'
  | 'audit_stream_removed'
  | 'vpn_client_connected'
  | 'vpn_client_disconnected';

export const auditEventTypeValues: AuditEventType[] = [
  'user_login',
  'user_logout',
  'user_added',
  'user_modified',
  'user_removed',
  'mfa_disabled',
  'mfa_totp_enabled',
  'mfa_totp_disabled',
  'mfa_email_enabled',
  'mfa_email_disabled',
  'mfa_security_key_added',
  'mfa_security_key_removed',
  'device_added',
  'device_modified',
  'device_removed',
  'network_device_added',
  'network_device_modified',
  'network_device_removed',
  'audit_stream_created',
  'audit_stream_modified',
  'audit_stream_removed',
  'vpn_client_connected',
  'vpn_client_disconnected',
];
