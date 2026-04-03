import { toNumber } from 'lodash-es';

export const networkSize = (network_address: string): number => {
  let maximal_cidr = 0;
  for (const address of network_address.split(',')) {
    const cidr = toNumber(address.trim().split('/')[1]);
    if (cidr > maximal_cidr) {
      maximal_cidr = cidr;
    }
  }
  return 2 ** (32 - maximal_cidr) - 3;
};
