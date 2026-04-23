import ipaddr from 'ipaddr.js';

export const smallestNetworkCapacity = (network_address: string): number => {
  let minCapacity = Infinity;
  for (const address of network_address.split(',')) {
    const trimmed = address.trim();
    try {
      const [ip, prefix] = ipaddr.parseCIDR(trimmed);
      let capacity: number;
      if (ip.kind() === 'ipv4') {
        capacity = 2 ** (32 - prefix) - 3;
      } else {
        // IPv6 has no broadcast address, so overhead is 2 (network + gateway)
        const raw = 2 ** (128 - prefix) - 2;
        capacity = Math.min(raw, Number.MAX_SAFE_INTEGER);
      }
      if (capacity < minCapacity) {
        minCapacity = capacity;
      }
    } catch {
      // unparseable entry — skip it
    }
  }
  return minCapacity === Infinity ? 0 : minCapacity;
};
