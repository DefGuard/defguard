import { APIRequestContext, expect } from '@playwright/test';

import { testsConfig } from '../../config';

export const apiEnrollmentStart = async (request: APIRequestContext, token: string) => {
  const url = `${testsConfig.ENROLLMENT_URL}/api/v1/enrollment/start`;
  const response = await request.post(url, {
    data: { token },
    headers: {
      'Content-Type': 'application/json',
    },
  });
  return response.json();
};

export const apiEnrollmentActivateUser = async (
  request: APIRequestContext,
  password: string,
  phoneNumber?: string,
): Promise<void> => {
  const url = `${testsConfig.ENROLLMENT_URL}/api/v1/enrollment/activate_user`;
  await request.post(url, {
    data: {
      password,
      phone_number: phoneNumber,
    },
    headers: {
      'Content-Type': 'application/json',
    },
  });
};

export const apiEnrollmentCreateDevice = async (
  request: APIRequestContext,
  token: string,
  device: { name: string; pubkey: string },
) => {
  const startUrl = `${testsConfig.ENROLLMENT_URL}/api/v1/enrollment/start`;
  const startResponse = await request.post(startUrl, { data: { token } });
  expect(startResponse.status()).toBe(200);
  const createUrl = `${testsConfig.ENROLLMENT_URL}/api/v1/enrollment/create_device`;
  const createResponse = await request.post(createUrl, {
    data: { name: device.name, pubkey: device.pubkey, token: null },
  });
  expect(createResponse.status()).toBe(200);
  return createResponse.json();
};
