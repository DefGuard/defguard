import { objectify } from 'radashi';
import type { Resource, ResourceById } from '../api/types';
import { isPresent } from '../defguard-ui/utils/isPresent';

export const resourceById = <T extends Resource>(values?: T[]): ResourceById<T> | null =>
  isPresent(values)
    ? objectify(
        values,
        (item) => item.id,
        (item) => item,
      )
    : null;
