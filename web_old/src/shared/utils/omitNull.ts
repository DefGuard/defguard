import { omitBy } from 'lodash-es';

export const omitNull = <T extends object>(val?: T): Partial<T> =>
  omitBy(val, (v) => v === null);
