export const EXPIRING_THRESHOLD_DAYS = 30;

export const getDaysUntilExpiry = (value: string | null | undefined): number | null => {
  if (!value) return null;

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;

  const millisecondsPerDay = 1000 * 60 * 60 * 24;
  return Math.trunc((date.getTime() - Date.now()) / millisecondsPerDay);
};
