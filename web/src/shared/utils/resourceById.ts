import { objectify } from 'radashi';
import type { Resource, ResourceById, ResourceDisplay } from '../api/types';
import { isPresent } from '../defguard-ui/utils/isPresent';

export const resourceById = <T extends Resource>(values?: T[]): ResourceById<T> | null =>
  isPresent(values)
    ? objectify(
        values,
        (item) => item.id,
        (item) => item,
      )
    : null;

export const resourceByIdNotNull = <T extends Resource>(values: T[]): ResourceById<T> =>
  objectify(
    values,
    (item) => item.id,
    (item) => item,
  );

export const resourceDisplayMap = (values: ResourceDisplay[]): Record<number, string> =>
  objectify(
    values,
    (item) => item.id,
    (item) => item.display,
  );
