import type { SelectOption } from '../defguard-ui/components/Select/types';

export type NumericSelectOption = SelectOption<number>;

export type NumericSelectOptionMap = Readonly<Record<number, string>>;

/** Builds sorted numeric select options from a value-to-label map. */
export const createNumericSelectOptions = (
  optionMap: NumericSelectOptionMap,
): NumericSelectOption[] =>
  Object.entries(optionMap)
    .map(([value, label]) => {
      const numericValue = Number(value);

      return {
        key: numericValue,
        label,
        value: numericValue,
      };
    })
    .sort((a, b) => a.value - b.value);

/** Appends the current numeric value when it is missing from predefined options. */
export const withNumericFallbackOption = (
  options: readonly NumericSelectOption[],
  value: number,
  unit: string,
): NumericSelectOption[] => {
  if (options.some((option) => option.value === value)) {
    return [...options];
  }

  const label = `${value} ${unit}`;

  return [
    ...options,
    {
      key: value,
      label,
      value,
    },
  ].sort((a, b) => a.value - b.value);
};
