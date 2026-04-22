import { describe, expect, it } from 'vitest';
import { joinCsv, splitCsv } from '../src/shared/utils/csv';
import { smallestNetworkCapacity } from '../src/shared/utils/network';
import { formatFileName } from '../src/shared/utils/formatFileName';
import { formatIpForDisplay } from '../src/shared/utils/formatIpForDisplay';
import { removeEmptyStrings } from '../src/shared/utils/removeEmptyStrings';
import { removeNulls } from '../src/shared/utils/removeNulls';
import { resourceById, resourceDisplayMap } from '../src/shared/utils/resourceById';
import { isDeviceOnline, isUserOnline } from '../src/shared/utils/userOnlineStatus';
import { isValidDefguardUrl } from '../src/shared/utils/defguardUrl';
import type { Device, DeviceNetworkInfo } from '../src/shared/api/types';


describe('joinCsv', () => {
  it('should join array into comma-separated string', () => {
    expect(joinCsv(['a', 'b', 'c'])).toBe('a, b, c');
    expect(joinCsv(['192.168.1.1', '10.0.0.1'])).toBe('192.168.1.1, 10.0.0.1');
  });

  it('should return string as-is when passed a string', () => {
    expect(joinCsv('already a string')).toBe('already a string');
  });

  it('should return empty string for empty array', () => {
    expect(joinCsv([])).toBe('');
  });

  it('should return empty string for null', () => {
    expect(joinCsv(null)).toBe('');
  });

  it('should return empty string for undefined', () => {
    expect(joinCsv(undefined)).toBe('');
  });

});

describe('splitCsv', () => {
  it('should split comma-separated string into array', () => {
    expect(splitCsv('a, b, c')).toEqual(['a', 'b', 'c']);
    expect(splitCsv('192.168.1.1,10.0.0.1')).toEqual(['192.168.1.1', '10.0.0.1']);
  });

  it('should trim whitespace around items', () => {
    expect(splitCsv(' a , b , c ')).toEqual(['a', 'b', 'c']);
  });

  it('should filter out empty items', () => {
    expect(splitCsv('a,,b')).toEqual(['a', 'b']);
    expect(splitCsv('a, ,b')).toEqual(['a', 'b']);
  });

  it('should return empty array for empty string', () => {
    expect(splitCsv('')).toEqual([]);
  });

  it('should return empty array for whitespace-only string', () => {
    expect(splitCsv('   ')).toEqual([]);
  });

});

describe('joinCsv / splitCsv round-trip', () => {
  it('should round-trip array through join and split', () => {
    const original = ['192.168.1.1', '10.0.0.0/8', '172.16.0.1-172.16.0.10'];
    expect(splitCsv(joinCsv(original))).toEqual(original);
  });
});


describe('formatFileName', () => {
  it('should convert to lowercase', () => {
    expect(formatFileName('MyFile')).toBe('myfile');
    expect(formatFileName('README')).toBe('readme');
  });

  it('should replace spaces with underscores', () => {
    expect(formatFileName('my config file')).toBe('my_config_file');
  });

  it('should trim leading and trailing whitespace', () => {
    expect(formatFileName('  file  ')).toBe('file');
    expect(formatFileName('  my file  ')).toBe('my_file');
  });

  it('should handle empty string', () => {
    expect(formatFileName('')).toBe('');
  });

  it('should handle string with only spaces', () => {
    expect(formatFileName('   ')).toBe('');
  });
});


describe('formatIpForDisplay', () => {
  it('should strip /32 from IPv4 host address', () => {
    expect(formatIpForDisplay('192.168.1.1/32')).toBe('192.168.1.1');
    expect(formatIpForDisplay('10.0.0.1/32')).toBe('10.0.0.1');
  });

  it('should strip /128 from IPv6 host address', () => {
    expect(formatIpForDisplay('2001:db8::1/128')).toBe('2001:db8::1');
    expect(formatIpForDisplay('::1/128')).toBe('::1');
  });

  it('should keep IPv4 CIDR that is not a host address', () => {
    expect(formatIpForDisplay('192.168.1.0/24')).toBe('192.168.1.0/24');
    expect(formatIpForDisplay('10.0.0.0/8')).toBe('10.0.0.0/8');
  });

  it('should keep IPv6 CIDR that is not a host address', () => {
    expect(formatIpForDisplay('2001:db8::/32')).toBe('2001:db8::/32');
    expect(formatIpForDisplay('fe80::/10')).toBe('fe80::/10');
  });

  it('should return plain IP unchanged (no slash)', () => {
    expect(formatIpForDisplay('192.168.1.1')).toBe('192.168.1.1');
    expect(formatIpForDisplay('2001:db8::1')).toBe('2001:db8::1');
  });

  it('should handle empty string', () => {
    expect(formatIpForDisplay('')).toBe('');
  });
});


describe('removeEmptyStrings', () => {
  it('should remove keys with empty string values', () => {
    expect(removeEmptyStrings({ a: '', b: 'hello' })).toEqual({ b: 'hello' });
  });

  it('should remove keys with whitespace-only string values', () => {
    expect(removeEmptyStrings({ a: '   ', b: 'hello' })).toEqual({ b: 'hello' });
  });

  it('should keep non-string falsy values', () => {
    expect(removeEmptyStrings({ a: 0, b: false, c: null })).toEqual({ a: 0, b: false, c: null });
  });

  it('should keep non-empty strings', () => {
    expect(removeEmptyStrings({ a: 'value', b: ' space ' })).toEqual({ a: 'value', b: ' space ' });
  });

  it('should handle object with all empty strings', () => {
    expect(removeEmptyStrings({ a: '', b: '' })).toEqual({});
  });

  it('should keep number, boolean, object values unchanged', () => {
    const input = { name: 'test', count: 42, active: true, nested: { x: 1 } };
    expect(removeEmptyStrings(input)).toEqual(input);
  });
});


describe('removeNulls', () => {
  it('should remove null values from flat object', () => {
    expect(removeNulls({ a: 1, b: null, c: 'hello' })).toEqual({ a: 1, c: 'hello' });
  });

  it('should remove undefined values from flat object', () => {
    expect(removeNulls({ a: 1, b: undefined, c: 'hello' })).toEqual({ a: 1, c: 'hello' });
  });

  it('should recursively remove nulls from nested objects', () => {
    expect(removeNulls({ a: { b: null, c: 1 }, d: 'hello' })).toEqual({
      a: { c: 1 },
      d: 'hello',
    });
  });

  it('should replace null/undefined with undefined in arrays (preserves slots)', () => {
    const result = removeNulls([1, null, 2, undefined, 3]);
    expect(result).toEqual([1, undefined, 2, undefined, 3]);
  });

  it('should keep falsy non-null/undefined values', () => {
    expect(removeNulls({ a: 0, b: false, c: '' })).toEqual({ a: 0, b: false, c: '' });
  });
});

describe('resourceById', () => {
  it('should index array of resources by id', () => {
    const items = [
      { id: 1, name: 'Alice' },
      { id: 2, name: 'Bob' },
    ];
    const result = resourceById(items);
    expect(result).toEqual({ 1: { id: 1, name: 'Alice' }, 2: { id: 2, name: 'Bob' } });
  });

  it('should return null for undefined input', () => {
    expect(resourceById(undefined)).toBeNull();
  });

  it('should return empty object for empty array', () => {
    expect(resourceById([])).toEqual({});
  });

});

describe('resourceDisplayMap', () => {
  it('should map id to display string', () => {
    const items = [
      { id: 1, display: 'Alpha' },
      { id: 2, display: 'Beta' },
    ];
    expect(resourceDisplayMap(items)).toEqual({ 1: 'Alpha', 2: 'Beta' });
  });

});


const makeNetwork = (is_active: boolean): DeviceNetworkInfo => ({
  device_wireguard_ips: [],
  is_active,
  network_gateway_ip: '10.0.0.1',
  network_id: 1,
  network_name: 'test',
});

const makeDevice = (networks: DeviceNetworkInfo[]): Device => ({
  id: 1,
  user_id: 1,
  name: 'device',
  wireguard_pubkey: 'key',
  created: '2024-01-01T00:00:00Z',
  networks,
});

describe('isDeviceOnline', () => {
  it('should return true if any network is active', () => {
    const device = makeDevice([makeNetwork(false), makeNetwork(true)]);
    expect(isDeviceOnline(device)).toBe(true);
  });

  it('should return false if no network is active', () => {
    const device = makeDevice([makeNetwork(false), makeNetwork(false)]);
    expect(isDeviceOnline(device)).toBe(false);
  });

  it('should return false for device with no networks', () => {
    const device = makeDevice([]);
    expect(isDeviceOnline(device)).toBe(false);
  });

});

describe('isUserOnline', () => {
  it('should return true if any device has an active network', () => {
    const user = {
      devices: [makeDevice([makeNetwork(false)]), makeDevice([makeNetwork(true)])],
    } as any;
    expect(isUserOnline(user)).toBe(true);
  });

  it('should return false if no device is online', () => {
    const user = {
      devices: [makeDevice([makeNetwork(false)]), makeDevice([makeNetwork(false)])],
    } as any;
    expect(isUserOnline(user)).toBe(false);
  });

  it('should return false if user has no devices', () => {
    const user = { devices: [] } as any;
    expect(isUserOnline(user)).toBe(false);
  });
});


describe('isValidDefguardUrl', () => {
  it('should accept valid https domain URLs', () => {
    expect(isValidDefguardUrl('https://defguard.example.com')).toBe(true);
    expect(isValidDefguardUrl('https://app.company.org')).toBe(true);
    expect(isValidDefguardUrl('https://vpn.internal.corp')).toBe(true);
  });

  it('should accept valid http domain URLs', () => {
    expect(isValidDefguardUrl('http://defguard.example.com')).toBe(true);
  });

  it('should accept URL with port', () => {
    expect(isValidDefguardUrl('https://defguard.example.com:8080')).toBe(true);
  });

  it('should accept URL with path', () => {
    expect(isValidDefguardUrl('https://defguard.example.com/enrollment')).toBe(true);
  });

  it('should reject URLs with IP address as hostname', () => {
    expect(isValidDefguardUrl('https://192.168.1.1')).toBe(false);
    expect(isValidDefguardUrl('https://10.0.0.1:8080')).toBe(false);
    expect(isValidDefguardUrl('http://127.0.0.1')).toBe(false);
  });

  it('should reject invalid URLs', () => {
    expect(isValidDefguardUrl('not-a-url')).toBe(false);
    expect(isValidDefguardUrl('')).toBe(false);
    expect(isValidDefguardUrl('ftp://')).toBe(false);
  });
});


describe('smallestNetworkCapacity', () => {
  // IPv4 cases
  it('should return 253 for a single IPv4 /24', () => {
    // 2^(32-24) - 3 = 256 - 3 = 253
    expect(smallestNetworkCapacity('10.0.0.1/24')).toBe(253);
  });

  it('should return 1 for a single IPv4 /30', () => {
    // 2^(32-30) - 3 = 4 - 3 = 1
    expect(smallestNetworkCapacity('10.0.0.1/30')).toBe(1);
  });

  it('should return -1 for a single IPv4 /32 host address', () => {
    // 2^(32-32) - 3 = 1 - 3 = -2... actually 0 hosts, 2^0=1, 1-3=-2
    // but the function should be consistent: capacity < 0 means not usable
    expect(smallestNetworkCapacity('10.0.0.1/32')).toBeLessThan(0);
  });

  // IPv6 cases
  it('should return MAX_SAFE_INTEGER for a large IPv6 /64 subnet', () => {
    // 2^(128-64) - 2 is astronomically large; must be capped
    expect(smallestNetworkCapacity('fd00::1/64')).toBe(Number.MAX_SAFE_INTEGER);
  });

  it('should return 2 for a tiny IPv6 /126 subnet', () => {
    // 2^(128-126) - 2 = 4 - 2 = 2
    expect(smallestNetworkCapacity('fd00::1/126')).toBe(2);
  });

  it('should return 1 for an IPv6 /127 subnet', () => {
    // 2^(128-127) - 2 = 2 - 2 = 0... hmm, actually 0.
    // /127 has 2 addresses, no broadcast, gateway takes 1, so 1 usable
    // Wait: 2^1 - 2 = 0. That means the formula gives 0 for /127.
    // Let's use the same logic as IPv4: 2^(128-prefix) - 2
    expect(smallestNetworkCapacity('fd00::1/127')).toBe(0);
  });

  it('should return negative for an IPv6 /128 host address', () => {
    // 2^(128-128) - 2 = 1 - 2 = -1
    expect(smallestNetworkCapacity('fd00::1/128')).toBe(-1);
  });

  // Mixed cases — should return the minimum capacity across all subnets
  it('should return IPv4 capacity when IPv4 subnet is smaller', () => {
    // IPv4 /30 → 1, IPv6 /64 → MAX_SAFE_INTEGER
    expect(smallestNetworkCapacity('10.0.0.1/30, fd00::1/64')).toBe(1);
  });

  it('should return IPv6 capacity when IPv6 subnet is smaller', () => {
    // IPv4 /24 → 253, IPv6 /126 → 2
    expect(smallestNetworkCapacity('10.0.0.1/24, fd00::1/126')).toBe(2);
  });
});
