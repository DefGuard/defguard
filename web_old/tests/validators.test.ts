import { describe, expect, it } from 'vitest';
import { Validate } from '../src/shared/validators';

describe('Validate.IPv4', () => {
  it('should accept valid IPv4 addresses', () => {
    expect(Validate.IPv4('192.168.1.1')).toBe(true);
    expect(Validate.IPv4('10.0.0.1')).toBe(true);
    expect(Validate.IPv4('172.16.0.1')).toBe(true);
    expect(Validate.IPv4('255.255.255.255')).toBe(true);
    expect(Validate.IPv4('0.0.0.0')).toBe(true);
  });

  it('should reject invalid IPv4 addresses', () => {
    expect(Validate.IPv4('1')).toBe(false);
    expect(Validate.IPv4('256.1.1.1')).toBe(false);
    expect(Validate.IPv4('192.168.1')).toBe(false);
    expect(Validate.IPv4('192.168.1.1.1')).toBe(false);
    expect(Validate.IPv4('abc.def.ghi.jkl')).toBe(false);
    expect(Validate.IPv4('192.168.1.1/24')).toBe(false);
  });

  it('should reject empty strings', () => {
    expect(Validate.IPv4('')).toBe(false);
  });
});

describe('Validate.IPv4withPort', () => {
  it('should accept valid IPv4 with port', () => {
    expect(Validate.IPv4withPort('192.168.1.1:8080')).toBe(true);
    expect(Validate.IPv4withPort('10.0.0.1:80')).toBe(true);
    expect(Validate.IPv4withPort('127.0.0.1:5051')).toBe(true);
    expect(Validate.IPv4withPort('192.168.1.1:65535')).toBe(true);
  });

  it('should reject IPv4 without port', () => {
    expect(Validate.IPv4withPort('192.168.1.1')).toBe(false);
  });

  it('should reject invalid port numbers', () => {
    expect(Validate.IPv4withPort('192.168.1.1:0')).toBe(false);
    expect(Validate.IPv4withPort('192.168.1.1:65536')).toBe(false);
    expect(Validate.IPv4withPort('192.168.1.1:99999')).toBe(false);
  });

  it('should reject invalid IPv4 format', () => {
    expect(Validate.IPv4withPort('256.1.1.1:8080')).toBe(false);
    expect(Validate.IPv4withPort('192.168.1:8080')).toBe(false);
  });
});

describe('Validate.IPv6', () => {
  it('should accept valid IPv6 addresses', () => {
    expect(Validate.IPv6('2001:db8::1')).toBe(true);
    expect(Validate.IPv6('::1')).toBe(true);
    expect(Validate.IPv6('::')).toBe(true);
    expect(Validate.IPv6('2001:0db8:0000:0000:0000:0000:0000:0001')).toBe(true);
    expect(Validate.IPv6('fe80::1')).toBe(true);
  });

  it('should reject invalid IPv6 addresses', () => {
    expect(Validate.IPv6('192.168.1.1')).toBe(false);
    expect(Validate.IPv6('gggg::1')).toBe(false);
    expect(Validate.IPv6('invalid')).toBe(false);
  });
});

describe('Validate.IPv6withPort', () => {
  it('should accept valid IPv6 with port in brackets', () => {
    expect(Validate.IPv6withPort('[::1]:8080')).toBe(true);
    expect(Validate.IPv6withPort('[2001:db8::1]:80')).toBe(true);
    expect(Validate.IPv6withPort('[fe80::1]:65535')).toBe(true);
  });

  it('should reject IPv6 without brackets', () => {
    expect(Validate.IPv6withPort('::1:8080')).toBe(false);
    expect(Validate.IPv6withPort('2001:db8::1:8080')).toBe(false);
  });

  it('should reject IPv6 without port', () => {
    expect(Validate.IPv6withPort('[::1]')).toBe(false);
  });

  it('should reject invalid port numbers', () => {
    expect(Validate.IPv6withPort('[::1]:0')).toBe(false);
    expect(Validate.IPv6withPort('[::1]:65536')).toBe(false);
  });
});

describe('Validate.CIDRv4', () => {
  it('should accept valid IPv4 CIDR notation', () => {
    expect(Validate.CIDRv4('192.168.1.0/24')).toBe(true);
    expect(Validate.CIDRv4('10.0.0.0/8')).toBe(true);
    expect(Validate.CIDRv4('172.16.0.0/12')).toBe(true);
    expect(Validate.CIDRv4('192.168.1.1/32')).toBe(true);
    expect(Validate.CIDRv4('192.168.1.0/1')).toBe(true);
  });

  it('should reject CIDR with /0 mask', () => {
    expect(Validate.CIDRv4('192.168.1.0/0')).toBe(false);
  });

  it('should reject invalid CIDR masks', () => {
    expect(Validate.CIDRv4('192.168.1.0/33')).toBe(false);
    expect(Validate.CIDRv4('192.168.1.0/99')).toBe(false);
  });

  it('should reject IPv4 without CIDR mask', () => {
    expect(Validate.CIDRv4('192.168.1.1')).toBe(false);
  });

  it('should reject invalid IPv4 in CIDR', () => {
    expect(Validate.CIDRv4('256.1.1.1/24')).toBe(false);
    expect(Validate.CIDRv4('192.168.1/24')).toBe(false);
  });
});

describe('Validate.CIDRv6', () => {
  it('should accept valid IPv6 CIDR notation', () => {
    expect(Validate.CIDRv6('2001:db8::/32')).toBe(true);
    expect(Validate.CIDRv6('fe80::/10')).toBe(true);
    expect(Validate.CIDRv6('::1/128')).toBe(true);
  });

  it('should reject CIDR with /0 mask', () => {
    expect(Validate.CIDRv6('2001:db8::/0')).toBe(false);
  });

  it('should reject invalid CIDR masks', () => {
    expect(Validate.CIDRv6('2001:db8::/129')).toBe(false);
  });

  it('should reject IPv6 without CIDR mask', () => {
    expect(Validate.CIDRv6('2001:db8::1')).toBe(false);
  });
});

describe('Validate.Domain', () => {
  it('should accept valid domain names', () => {
    expect(Validate.Domain('example.com')).toBe(true);
    expect(Validate.Domain('sub.example.com')).toBe(true);
    expect(Validate.Domain('my-domain.co.uk')).toBe(true);
    expect(Validate.Domain('test123.example.org')).toBe(true);
  });

  it('should reject domains with port', () => {
    expect(Validate.Domain('example.com:8080')).toBe(false);
  });

  it('should reject invalid domain formats', () => {
    expect(Validate.Domain('invalid domain')).toBe(false);
    expect(Validate.Domain('example..com')).toBe(false);
    expect(Validate.Domain('domain.secret.com')).toBe(true);
  });
});

describe('Validate.DomainWithPort', () => {
  it('should accept valid domains with port', () => {
    expect(Validate.DomainWithPort('example.com:8080')).toBe(true);
    expect(Validate.DomainWithPort('sub.example.com:443')).toBe(true);
    expect(Validate.DomainWithPort('test.org:3000')).toBe(true);
  });

  it('should reject domains without port', () => {
    expect(Validate.DomainWithPort('example.com')).toBe(false);
  });

  it('should reject invalid port numbers', () => {
    expect(Validate.DomainWithPort('example.com:0')).toBe(false);
    expect(Validate.DomainWithPort('example.com:65536')).toBe(false);
    expect(Validate.DomainWithPort('example.com:99999')).toBe(false);
  });
});

describe('Validate.Port', () => {
  it('should accept valid port numbers', () => {
    expect(Validate.Port('1')).toBe(true);
    expect(Validate.Port('80')).toBe(true);
    expect(Validate.Port('443')).toBe(true);
    expect(Validate.Port('8080')).toBe(true);
    expect(Validate.Port('65535')).toBe(true);
  });

  it('should reject port 0', () => {
    expect(Validate.Port('0')).toBe(false);
  });

  it('should reject ports above 65535', () => {
    expect(Validate.Port('65536')).toBe(false);
    expect(Validate.Port('99999')).toBe(false);
  });

  it('should reject non-numeric values', () => {
    expect(Validate.Port('abc')).toBe(false);
    expect(Validate.Port('12.34')).toBe(false);
    expect(Validate.Port('')).toBe(false);
  });

  it('should reject negative numbers', () => {
    expect(Validate.Port('-1')).toBe(false);
  });
});

describe('Validate.any', () => {
  it('should accept single valid value matching any validator', () => {
    expect(Validate.any('192.168.1.1', [Validate.IPv4, Validate.IPv6])).toBe(true);
    expect(Validate.any('2001:db8::1', [Validate.IPv4, Validate.IPv6])).toBe(true);
    expect(Validate.any('example.com', [Validate.Domain, Validate.IPv4])).toBe(true);
  });

  it('should reject single value not matching any validator', () => {
    expect(Validate.any('invalid', [Validate.IPv4, Validate.IPv6])).toBe(false);
    expect(Validate.any('256.1.1.1', [Validate.IPv4, Validate.IPv6])).toBe(false);
  });

  it('should reject multiple values when allowList is false (default)', () => {
    expect(Validate.any('192.168.1.1,10.0.0.1', [Validate.IPv4])).toBe(false);
    expect(Validate.any('example.com,test.com', [Validate.Domain])).toBe(false);
  });

  it('should accept multiple valid values when allowList is true', () => {
    expect(Validate.any('192.168.1.1,10.0.0.1', [Validate.IPv4], true)).toBe(true);
    expect(
      Validate.any('192.168.1.1,2001:db8::1', [Validate.IPv4, Validate.IPv6], true),
    ).toBe(true);
    expect(Validate.any('example.com,test.org', [Validate.Domain], true)).toBe(true);
  });

  it('should reject list with any invalid value when allowList is true', () => {
    expect(Validate.any('192.168.1.1,invalid', [Validate.IPv4], true)).toBe(false);
    expect(Validate.any('192.168.1.1,256.1.1.1', [Validate.IPv4], true)).toBe(false);
  });

  it('should handle mixed valid values with allowList', () => {
    expect(
      Validate.any(
        '192.168.1.1,2001:db8::1,10.0.0.1',
        [Validate.IPv4, Validate.IPv6],
        true,
      ),
    ).toBe(true);
    expect(
      Validate.any('example.com,192.168.1.1', [Validate.Domain, Validate.IPv4], true),
    ).toBe(true);
  });

  it('should handle custom split character', () => {
    expect(Validate.any('192.168.1.1;10.0.0.1', [Validate.IPv4], true, ';')).toBe(true);
    expect(Validate.any('192.168.1.1|10.0.0.1', [Validate.IPv4], true, '|')).toBe(true);
  });

  it('should handle whitespace in list', () => {
    expect(Validate.any('192.168.1.1, 10.0.0.1', [Validate.IPv4], true)).toBe(true);
    expect(Validate.any('192.168.1.1 , 10.0.0.1', [Validate.IPv4], true)).toBe(true);
  });

  it('should accept empty string with Empty validator in list', () => {
    expect(Validate.any('', [Validate.IPv4, Validate.Empty], true)).toBe(true);
  });
});

describe('Validate.all', () => {
  it('should accept single value matching all validators', () => {
    expect(Validate.all('192.168.1.1', [Validate.IPv4])).toBe(true);
  });

  it('should reject single value not matching all validators', () => {
    expect(Validate.all('192.168.1.1', [Validate.IPv4, Validate.IPv6])).toBe(false);
    expect(Validate.all('invalid', [Validate.IPv4])).toBe(false);
  });

  it('should accept empty string or undefined', () => {
    expect(Validate.all('', [Validate.IPv4])).toBe(true);
    expect(Validate.all(undefined, [Validate.IPv4])).toBe(true);
  });

  it('should reject multiple values when allowList is false (default)', () => {
    expect(Validate.all('192.168.1.1,10.0.0.1', [Validate.IPv4])).toBe(false);
  });

  it('should accept multiple valid values when allowList is true', () => {
    expect(Validate.all('192.168.1.1,10.0.0.1', [Validate.IPv4], true)).toBe(true);
    expect(Validate.all('example.com,test.org', [Validate.Domain], true)).toBe(true);
  });

  it('should reject if any value does not match all validators when allowList is true', () => {
    expect(Validate.all('192.168.1.1,invalid', [Validate.IPv4], true)).toBe(false);
    expect(Validate.all('192.168.1.1,256.1.1.1', [Validate.IPv4], true)).toBe(false);
  });

  it('should handle custom split character', () => {
    expect(Validate.all('192.168.1.1;10.0.0.1', [Validate.IPv4], true, ';')).toBe(true);
    expect(Validate.all('192.168.1.1|10.0.0.1', [Validate.IPv4], true, '|')).toBe(true);
  });

  it('should handle whitespace in list', () => {
    expect(Validate.all('192.168.1.1, 10.0.0.1', [Validate.IPv4], true)).toBe(true);
    expect(
      Validate.all('192.168.1.1 , 10.0.0.1 , 172.16.0.1', [Validate.IPv4], true),
    ).toBe(true);
  });
});
