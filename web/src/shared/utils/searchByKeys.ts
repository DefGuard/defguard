import { diceCoefficient } from 'dice-coefficient';
// Search in object by multiple keys, this assumes that value under given keys is a string
export const searchByKeys = <T extends object>(
  obj: T,
  searchedKeys: Array<keyof T>,
  searchValue: string,
): boolean => {
  if (searchValue === '') return true;
  for (const key of searchedKeys) {
    if (typeof obj[key] !== 'string') {
      throw Error(
        `Usage of searchByKeys is allowed only on values of type string! Value under key ${key.toString()} is not a string.`,
      );
    }
    const val = obj[key] as string;
    const loweredValue = val.toLowerCase();
    const loweredSearch = searchValue.toLowerCase();
    const score = diceCoefficient(loweredValue, loweredSearch);
    const includes = loweredValue.includes(loweredSearch);
    if (score >= 0.6 || includes) {
      return true;
    }
  }
  return false;
};
