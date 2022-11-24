import { permutations } from 'itertools';

const possibleCombinations = 1320;
const options = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
const allPermutations = Array.from(permutations(options, 3));


export const getDeviceAvatar = (id: number) => {
  const avatar = id % possibleCombinations;
  return allPermutations[avatar];
};
