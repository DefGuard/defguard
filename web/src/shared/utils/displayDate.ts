import dayjs from 'dayjs';

const defaultFormat = 'DD/MM/YYYY | HH:mm';

export const displayDate = (date: string | number) => {
  if (typeof date === 'number') {
    return dayjs.unix(date).local().format(defaultFormat);
  }
  return dayjs.utc(date).local().format(defaultFormat);
};
