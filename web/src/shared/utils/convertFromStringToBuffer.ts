export const convertFromStringToBuffer = (value: string): ArrayBuffer => {
  // Encode with UTF-8
  const encoder = new TextEncoder();
  return encoder.encode(value);
};
