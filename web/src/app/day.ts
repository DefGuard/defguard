import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
import 'dayjs/locale/en';
import localizedFormat from 'dayjs/plugin/localizedFormat';
import utc from 'dayjs/plugin/utc';
import { getLocale } from '../paraglide/runtime';

dayjs.extend(relativeTime);
dayjs.extend(utc);
dayjs.extend(localizedFormat);
dayjs.locale(getLocale());
