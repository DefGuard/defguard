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
  | 'device_added'
  | 'device_modified'
  | 'device_removed';

export const auditEventTypeValues: AuditEventType[] = [
  'user_login',
  'user_logout',
  'device_added',
  'device_modified',
  'device_removed',
];
