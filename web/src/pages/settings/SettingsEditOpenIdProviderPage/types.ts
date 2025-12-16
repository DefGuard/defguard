import type {
  AddOpenIdProvider,
  OpenIdProvider,
  OpenIdProviderSettings,
} from '../../../shared/api/types';

export interface EditProviderFormProps {
  provider: OpenIdProvider & OpenIdProviderSettings;
  onSubmit: (value: Partial<AddOpenIdProvider>) => Promise<void>;
  onDelete: () => void;
  loading: boolean;
}
