import ipaddr from 'ipaddr.js';

const isHostCidr = (value: string): boolean => {
  try {
    const [address, prefixLength] = ipaddr.parseCIDR(value);

    if (address.kind() === 'ipv4') {
      return prefixLength === 32;
    }

    if (address.kind() === 'ipv6') {
      return prefixLength === 128;
    }

    return false;
  } catch {
    return false;
  }
};

export const formatIpForDisplay = (value: string): string => {
  const separatorIndex = value.lastIndexOf('/');

  if (separatorIndex === -1) {
    return value;
  }

  if (!isHostCidr(value)) {
    return value;
  }

  return value.slice(0, separatorIndex);
};
