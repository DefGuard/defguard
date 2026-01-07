import { createFileRoute } from '@tanstack/react-router';
import { SettingsEditOpenIdProviderPage } from '../../../../pages/settings/SettingsEditOpenIdProviderPage/SettingsEditOpenIdProviderPage';
import { getExternalProviderQueryOptions } from '../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/settings/edit-openid')({
  component: SettingsEditOpenIdProviderPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getExternalProviderQueryOptions);
  },
});
