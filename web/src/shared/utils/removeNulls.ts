// biome-ignore lint/suspicious/noExplicitAny: should be like this
export const removeNulls = (obj: any) => {
  // eslint-disable-next-line @typescript-eslint/no-unsafe-return
  return JSON.parse(JSON.stringify(obj), (_, value) => {
    if (value == null) return undefined;
    // eslint-disable-next-line @typescript-eslint/no-unsafe-return
    return value;
  });
};
