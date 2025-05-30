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
  | 'mfa_enabled'
  | 'mfa_disabled'
  | 'device_added'
  | 'device_modified'
  | 'device_removed';

export const auditEventTypeValues: AuditEventType[] = [
  'user_login',
  'user_logout',
  'user_added',
  'user_modified',
  'user_removed',
  'mfa_enabled',
  'mfa_disabled',
  'device_added',
  'device_modified',
  'device_removed',
];
