import { m } from '../../../paraglide/messages';
import type { OpenIdProviderKindValue } from '../../api/types';
import { InfoBanner } from '../../defguard-ui/components/InfoBanner/InfoBanner';

interface Props {
  provider: OpenIdProviderKindValue;
}

export const ProviderUsersSyncWarning = ({ provider }: Props) => {
  return (
    <InfoBanner
      icon="info-outlined"
      variant="warning"
      text={m.cmp_provider_users_sync_warning({
        provider,
      })}
    />
  );
};
