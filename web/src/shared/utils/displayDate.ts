import dayjs, { type Dayjs } from 'dayjs';

export const dateToLocal = (value: string): Dayjs => dayjs.utc(value).local();
