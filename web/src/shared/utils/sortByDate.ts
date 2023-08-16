import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';

dayjs.extend(utc);

/**
 * Sorts array of objects by date field that came from core
 **/
export const sortByDate = <T extends object>(
  items: T[],
  extraction: (item: T) => string,
  descending = false,
): T[] => {
  return items.sort((itemA, itemB) => {
    const dateA = dayjs.utc(extraction(itemA)).toDate().getTime();
    const dateB = dayjs.utc(extraction(itemB)).toDate().getTime();
    if (descending) {
      return dateB - dateA;
    }
    return dateA - dateB;
  });
};
