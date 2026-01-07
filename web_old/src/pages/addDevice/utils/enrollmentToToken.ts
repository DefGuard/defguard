import { fromUint8Array } from 'js-base64';

export type EnrollmentData = {
  url: string;
  token: string;
};

export const enrollmentToImportToken = (url: string, token: string): string => {
  const data: EnrollmentData = {
    token,
    url,
  };
  const jsonString = JSON.stringify(data);
  const textEncoder = new TextEncoder();
  const encoded = textEncoder.encode(jsonString);
  return fromUint8Array(encoded);
};
