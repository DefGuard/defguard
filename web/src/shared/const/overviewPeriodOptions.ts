import type { SelectOption } from '../defguard-ui/components/Select/types';

export const overviewPeriodOptions: SelectOption<number>[] = [
  { key: 1, label: '1h period', value: 1 },
  { key: 2, label: '2h period', value: 2 },
  { key: 6, label: '6h period', value: 6 },
  { key: 8, label: '8h period', value: 8 },
  { key: 12, label: '12h period', value: 12 },
  { key: 16, label: '16h period', value: 16 },
  { key: 24, label: '24h period', value: 24 },
];
