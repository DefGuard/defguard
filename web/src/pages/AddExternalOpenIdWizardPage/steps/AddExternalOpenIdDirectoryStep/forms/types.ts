import type { AddOpenIdProvider } from '../../../../../shared/api/types';

export interface ProviderFormProps {
  onSubmit: (values: Partial<AddOpenIdProvider>) => Promise<void>;
}
