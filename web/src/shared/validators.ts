import { patternValidDomain, patternValidIp } from './patterns';

// Returns flase when invalid
export const validateIpOrDomain = (val: string, allowMask = false): boolean => {
  return validateIp(val, allowMask) || patternValidDomain.test(val);
};

// Returns flase when invalid
export const validateIpList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  const trimed = val.replace(' ', '');
  const split = trimed.split(splitWith);
  for (const value of split) {
    if (!validateIp(value, allowMasks)) {
      return false;
    }
  }
  return true;
};

// Returns flase when invalid
export const validateIpOrDomainList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  const trimed = val.replace(' ', '');
  const split = trimed.split(splitWith);
  for (const value of split) {
    if (!(validateIp(value, allowMasks) && patternValidDomain.test(value))) {
      return false;
    }
  }
  return true;
};

// Returns flase when invalid
export const validateIp = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.includes('/')) {
      const split = ip.split('/');
      if (split.length !== 2) return true;
      const ipValid = patternValidIp.test(split[0]);
      if (split[1] === '') return false;
      const mask = Number(split[1]);
      const maskValid = mask >= 0 && mask <= 32;
      return ipValid && maskValid;
    }
  }
  return patternValidIp.test(ip);
};

export const validatePort = (val: string) => {
  const parsed = parseInt(val);
  if (!isNaN(parsed)) {
    return parsed <= 65535;
  }
};

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);
