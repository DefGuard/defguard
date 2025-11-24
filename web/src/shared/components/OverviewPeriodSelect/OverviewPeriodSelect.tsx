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

type Props = {
  period: number;
  onChange: (period: number) => void;
};

export const OverviewPeriodSelect = ({ period, onChange }: Props) => {
  return (
    <Select
      value={periodToOption(period)}
      options={options}
      onChange={(option) => {
        onChange(option.value);
      }}
    />
  );
};
