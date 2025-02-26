// Search in object by multiple keys, this assumes that value under given keys is a string
export const searchByKeys = <T extends object>(
  obj: T,
  searchedKeys: Array<keyof T>,
  searchValue: string,
): boolean => {
  searchedKeys.forEach((key) => {
    if (typeof obj[key] !== 'string') {
      throw Error(
        `Usage of searchByKeys is allowed only on values of type string! Value under key ${key.toString()} is not a string.`,
      );
    }
    const val = obj[key] as string;
    if (val.toLocaleLowerCase().trim().includes(searchValue.trim().toLowerCase())) {
      return true;
    }
  });
  return false;
};
