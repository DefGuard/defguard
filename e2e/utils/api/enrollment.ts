import { APIRequestContext } from '@playwright/test';

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
