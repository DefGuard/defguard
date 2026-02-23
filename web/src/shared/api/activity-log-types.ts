import { m } from '../../paraglide/messages';

export const ActivityLogModule = {
  Defguard: 'defguard',
  Client: 'client',
  Vpn: 'vpn',
  Enrollment: 'enrollment',
} as const;

export type ActivityLogModuleValue =
  (typeof ActivityLogModule)[keyof typeof ActivityLogModule];

export const ActivityLogEventType = {
  RecoveryCodeUsed: 'recovery_code_used',

  UserLogin: 'user_login',
  UserLoginFailed: 'user_login_failed',
  UserMfaLogin: 'user_mfa_login',
  UserMfaLoginFailed: 'user_mfa_login_failed',
  UserLogout: 'user_logout',
  UserAdded: 'user_added',
  UserModified: 'user_modified',
  UserRemoved: 'user_removed',
  UserGroupsModified: 'user_groups_modified',

  MfaDisabled: 'mfa_disabled',
  UserMfaDisabled: 'user_mfa_disabled',
  MfaTotpEnabled: 'mfa_totp_enabled',
  MfaTotpDisabled: 'mfa_totp_disabled',
  MfaEmailEnabled: 'mfa_email_enabled',
  MfaEmailDisabled: 'mfa_email_disabled',
  MfaSecurityKeyAdded: 'mfa_security_key_added',
  MfaSecurityKeyRemoved: 'mfa_security_key_removed',

  DeviceAdded: 'device_added',
  DeviceModified: 'device_modified',
  DeviceRemoved: 'device_removed',

  NetworkDeviceAdded: 'network_device_added',
  NetworkDeviceModified: 'network_device_modified',
  NetworkDeviceRemoved: 'network_device_removed',

  ActivityLogStreamCreated: 'activity_log_stream_created',
  ActivityLogStreamModified: 'activity_log_stream_modified',
  ActivityLogStreamRemoved: 'activity_log_stream_removed',

  VpnClientConnected: 'vpn_client_connected',
  VpnClientDisconnected: 'vpn_client_disconnected',
  VpnClientMfaSuccess: 'vpn_client_mfa_success',
  VpnClientMfaFailed: 'vpn_client_mfa_failed',

  EnrollmentTokenAdded: 'enrollment_token_added',
  EnrollmentStarted: 'enrollment_started',
  EnrollmentDeviceAdded: 'enrollment_device_added',
  EnrollmentCompleted: 'enrollment_completed',

  PasswordResetRequested: 'password_reset_requested',
  PasswordResetStarted: 'password_reset_started',
  PasswordResetCompleted: 'password_reset_completed',

  VpnLocationAdded: 'vpn_location_added',
  VpnLocationRemoved: 'vpn_location_removed',
  VpnLocationModified: 'vpn_location_modified',

  ApiTokenAdded: 'api_token_added',
  ApiTokenRemoved: 'api_token_removed',
  ApiTokenRenamed: 'api_token_renamed',

  OpenIdAppAdded: 'open_id_app_added',
  OpenIdAppRemoved: 'open_id_app_removed',
  OpenIdAppModified: 'open_id_app_modified',
  OpenIdAppStateChanged: 'open_id_app_state_changed',
  OpenIdProviderRemoved: 'open_id_provider_removed',
  OpenIdProviderModified: 'open_id_provider_modified',

  SettingsUpdated: 'settings_updated',
  SettingsUpdatedPartial: 'settings_updated_partial',
  SettingsDefaultBrandingRestored: 'settings_default_branding_restored',

  GroupsBulkAssigned: 'groups_bulk_assigned',
  GroupAdded: 'group_added',
  GroupModified: 'group_modified',
  GroupRemoved: 'group_removed',
  GroupMemberAdded: 'group_member_added',
  GroupMemberRemoved: 'group_member_removed',
  GroupMembersModified: 'group_members_modified',

  WebHookAdded: 'web_hook_added',
  WebHookModified: 'web_hook_modified',
  WebHookRemoved: 'web_hook_removed',
  WebHookStateChanged: 'web_hook_state_changed',

  AuthenticationKeyAdded: 'authentication_key_added',
  AuthenticationKeyRemoved: 'authentication_key_removed',
  AuthenticationKeyRenamed: 'authentication_key_renamed',

  PasswordChanged: 'password_changed',
  PasswordChangedByAdmin: 'password_changed_by_admin',
  PasswordReset: 'password_reset',

  ClientConfigurationTokenAdded: 'client_configuration_token_added',

  UserSnatBindingAdded: 'user_snat_binding_added',
  UserSnatBindingModified: 'user_snat_binding_modified',
  UserSnatBindingRemoved: 'user_snat_binding_removed',

  ProxyModified: 'proxy_modified',
  ProxyDeleted: 'proxy_deleted',

  GatewayModified: 'gateway_modified',
  GatewayDeleted: 'gateway_deleted',
} as const;

export type ActivityLogEventTypeValue =
  (typeof ActivityLogEventType)[keyof typeof ActivityLogEventType];

export const activityLogEvents = Object.keys(ActivityLogEventType) as Array<
  keyof typeof ActivityLogEventType
>;

const valueToTranslation = () => {
  return Object.values(ActivityLogEventType).reduce(
    (acc, event) => {
      acc[event] = m[`activity_event_${event}`]();
      return acc;
    },
    {} as Record<ActivityLogEventTypeValue, string>,
  );
};

export const activityLogEventDisplay: Record<ActivityLogEventTypeValue, string> =
  valueToTranslation();

export const activityLogEventsSet = new Set(activityLogEvents);
