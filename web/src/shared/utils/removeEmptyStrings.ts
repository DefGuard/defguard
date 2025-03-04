export function removeEmptyStringKeys<T extends Record<string, unknown>>(
  obj: T,
): Partial<T> {
  return Object.fromEntries(
    Object.entries(obj).filter(
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      ([_, value]) => !(typeof value === 'string' && value === ''),
    ),
  ) as Partial<T>;
}
