export const titleCase = (str: string): string => {
  const res = str.toLowerCase().split(' ');
  return res.map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
};
