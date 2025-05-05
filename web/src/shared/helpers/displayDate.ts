import dayjs from 'dayjs';

/**
 * Parse date from Core API to readable standarized date to display for user to see.
 * **/
export const displayDate = (dateFromApi: string): string => {
  return dayjs.utc(dateFromApi).format('DD.MM.YYYY');
};
