export const joinCsv = (values?: string[] | string | null): string => {
  if (!values) return '';
  if (typeof values === 'string') return values;
  if (values.length === 0) return '';
  return values.join(', ');
};

export const toCsvArray = (value?: string[] | string | null): string[] | null => {
  if (Array.isArray(value)) return value;
  if (value) return [value];
  return null;
};

export const splitCsv = (value: string): string[] => {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
};
