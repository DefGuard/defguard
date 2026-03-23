import dayjs from 'dayjs';

const defaultFormat = 'DD/MM/YYYY | HH:mm';

export const displayDate = (date: string | number, format = defaultFormat) => {
  if (typeof date === 'number') {
    return dayjs.unix(date).local().format(format);
  }
  return dayjs.utc(date).local().format(format);
};
