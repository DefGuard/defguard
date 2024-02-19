/**Search for strings in object and trims them, designed for preparing form values to be sent to backend */
export const trimObjectStrings = <T extends object>(obj: T): T => {
  if (typeof obj !== 'object' || obj === null) {
    return obj;
  }

  if (Array.isArray(obj)) {
    return obj.map((item) => trimObjectStrings(item)) as unknown as T;
  }

  const trimmedObj: Record<string, unknown> = {};

  Object.entries(obj).forEach(([key, value]) => {
    if (typeof value === 'string') {
      trimmedObj[key] = value.trim();
    } else if (typeof value === 'object' && value !== null) {
      trimmedObj[key] = trimObjectStrings(value);
    } else {
      trimmedObj[key] = value;
    }
  });

  return trimmedObj as T;
};
