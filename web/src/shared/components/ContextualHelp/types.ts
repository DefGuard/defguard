export const ContextualHelpKey = {
  // Main Settings tabs
  SettingsGeneral: 'settings-general',
  SettingsNotifications: 'settings-notifications',
  SettingsIdentity: 'settings-identity',
  SettingsActivity: 'settings-activity',
  SettingsCerts: 'settings-certs',
  SettingsLicense: 'settings-license',
  // Settings subpages
  SettingsInstance: 'settings-instance',
  SettingsClient: 'settings-client',
  SettingsSmtp: 'settings-smtp',
  SettingsGatewayNotifications: 'settings-gateway-notifications',
  SettingsOpenId: 'settings-openid',
  SettingsLdap: 'settings-ldap',
  SettingsCa: 'settings-ca',
  SettingsEdgeCoreCerts: 'settings-edge-core-certs',
  // Enrollment settings tabs
  EnrollmentGeneral: 'enrollment-general',
  EnrollmentMessageTemplates: 'enrollment-message-templates',
  // Support page
  Support: 'support',
} as const;

export type ContextualHelpKey =
  (typeof ContextualHelpKey)[keyof typeof ContextualHelpKey];

export interface ContextualHelpFaq {
  question: string;
  answer: string;
}

export interface ContextualHelpDoc {
  title: string;
  url: string;
}

export interface ContextualHelpPage {
  faqs?: ContextualHelpFaq[];
  relatedDocs?: ContextualHelpDoc[];
  bestPractices?: string;
}

export interface ContextualHelpVersionEntry {
  pages: Record<string, ContextualHelpPage>;
}

export type ContextualHelpMappings = Record<string, ContextualHelpVersionEntry>;
