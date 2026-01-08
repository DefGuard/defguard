import z from 'zod';

export const enumFromObjValues = <T extends Record<string, V>, V extends string>(
  obj: T,
) => z.enum(Object.values(obj) as [V, ...V[]]);
