import type { SelectOption } from '../defguard-ui/components/Select/types';

export type NumericSelectOption = SelectOption<number>;

export const formatDaySelectLabel = (value: number) =>
  `${value} ${value === 1 ? 'day' : 'days'}`;

export const formatHourSelectLabel = (value: number) =>
  `${value} ${value === 1 ? 'hour' : 'hours'}`;

export const formatMinuteSelectLabel = (value: number) =>
  `${value} ${value === 1 ? 'minute' : 'minutes'}`;

export const createNumericSelectOptions = (
  values: readonly number[],
  formatLabel: (value: number) => string,
): NumericSelectOption[] =>
  values.map((value) => ({
    key: value,
    label: formatLabel(value),
    value,
  }));

export const withNumericFallbackOption = (
  options: readonly NumericSelectOption[],
  value: number,
  formatLabel: (value: number) => string,
): NumericSelectOption[] => {
  if (options.some((option) => option.value === value)) {
    return [...options];
  }

  return [
    ...options,
    {
      key: value,
      label: formatLabel(value),
      value,
    },
  ].sort((a, b) => a.value - b.value);
};
