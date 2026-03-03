export const joinCsv = (values?: string[] | null): string => {
  if (!values || values.length === 0) return '';
  return values.join(', ');
};

export const splitCsv = (value: string): string[] => {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
};
