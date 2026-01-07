import type { SelectOption } from '../../defguard-ui/components/Layout/Select/types';
import type { Network } from '../../types';

export const selectifyNetworks = (data: Network[]): SelectOption<number>[] =>
  data.map((network) => ({
    key: network.id,
    label: network.name,
    value: network.id,
  }));
