export const formatFileName = (value: string) =>
  value.trim().replaceAll(' ', '_').toLowerCase();
