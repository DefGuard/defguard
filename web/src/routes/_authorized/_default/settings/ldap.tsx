import { createFileRoute } from '@tanstack/react-router';
import { SettingsLdapPage } from '../../../../pages/settings/SettingsLdapPage/SettingsLdapPage';

export const Route = createFileRoute('/_authorized/_default/settings/ldap')({
  component: SettingsLdapPage,
});
