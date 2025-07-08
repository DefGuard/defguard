import { fromUint8Array } from 'js-base64';

export type EnrollmentData = {
  url: string;
  token: string;
};

const useLocalProxy = import.meta.env.DEV;

const extractProxyPort = (input: string): string | undefined => {
  try {
    const url = new URL(input);
    const port = url.port;
    const parsed = port ? parseInt(port, 10) : undefined;
    if (parsed && !isNaN(parsed)) {
      return `:${parsed}`;
    }
    return undefined;
  } catch {
    return undefined;
  }
};

export const enrollmentToImportToken = (url: string, token: string): string => {
  let proxyUrl: string;
  if (useLocalProxy) {
    const port = extractProxyPort(url);
    proxyUrl = `http://10.0.2.2${port}`;
  } else {
    proxyUrl = url;
  }
  const data: EnrollmentData = {
    token,
    url: proxyUrl,
  };
  const jsonString = JSON.stringify(data);
  const textEncoder = new TextEncoder();
  const encoded = textEncoder.encode(jsonString);
  return fromUint8Array(encoded);
};
