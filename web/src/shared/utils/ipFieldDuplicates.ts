export type IpFieldEntry = {
  path: string;
  ip: string;
};

export const getDuplicateIpFieldPaths = (entries: IpFieldEntry[]): Set<string> => {
  const firstPathByIp = new Map<string, string>();
  const duplicatePaths = new Set<string>();
  for (const entry of entries) {
    if (entry.ip.length === 0) continue;
    const firstPath = firstPathByIp.get(entry.ip);
    if (firstPath === undefined) {
      firstPathByIp.set(entry.ip, entry.path);
    } else {
      duplicatePaths.add(entry.path);
    }
  }
  return duplicatePaths;
};
