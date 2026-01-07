export const removeEmptyStrings = <T extends object>(obj: T): T => {
  const trimmed: T = {} as T;
  for (const [key, value] of Object.entries(obj)) {
    // Check if value is a string and empty
    if (typeof value === 'string' && value.trim() === '') {
      continue; // skip this key
    }
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    trimmed[key as keyof T] = value;
  }
  return trimmed;
};
