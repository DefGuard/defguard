export const isPresent = <T>(value: T): value is NonNullable<T> => {
  return value !== null && value !== undefined;
};
