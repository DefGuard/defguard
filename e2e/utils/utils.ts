export const mergeObjects = <T extends object>(
  partialObj: Partial<T>,
  defaultObj: T
): T => {
  const res: T = defaultObj;
  for (const key of Object.keys(defaultObj)) {
    if (partialObj.hasOwnProperty(key)) {
      if (typeof partialObj[key] !== 'undefined') {
        res[key] = partialObj[key];
      }
    }
  }

  return res;
};
