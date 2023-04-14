/* eslint-disable @typescript-eslint/no-explicit-any */
export const removeNulls = (obj: any) => {
  return JSON.parse(JSON.stringify(obj), (_, value) => {
    if (value == null) return undefined;
    return value;
  });
};
