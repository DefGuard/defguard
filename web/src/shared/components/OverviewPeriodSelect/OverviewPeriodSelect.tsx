import { useNavigate, useSearch } from '@tanstack/react-router';
import { Select } from '../../defguard-ui/components/Select/Select';
import type { SelectOption } from '../../defguard-ui/components/Select/types';

const periods = [1, 2, 4, 6, 8, 12, 16, 24];

const periodToOption = (value: number): SelectOption<number> => ({
  key: value,
  value: value,
  label: `${value} period`,
});

const options = periods.map((item) => ({
  key: item,
  value: item,
  label: `${item} period`,
}));

export const OverviewPeriodSelect = () => {
  const navigate = useNavigate({ from: '/vpn-overview' });
  const { period } = useSearch({ from: '/_authorized/vpn-overview/' });
  return (
    <Select
      value={periodToOption(period)}
      options={options}
      onChange={(option) => {
        navigate({
          search: {
            period: option.value,
          },
        });
      }}
    />
  );
};
