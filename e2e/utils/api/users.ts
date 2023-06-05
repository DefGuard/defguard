import { Page } from '@playwright/test';

import { testsConfig } from '../../config';
import { ApiUser, User } from '../../types';

export const apiGetUsers = async (page: Page): Promise<ApiUser[]> => {
  const url = testsConfig.CORE_BASE_URL + '/user';
  const users = await page.evaluate(async (url) => {
    return await fetch(url, {
      method: 'GET',
    }).then((res) => res.json());
  }, url);
  return users;
};

export const apiCreateUsersBulk = async (page: Page, users: User[]): Promise<void> => {
  const url = testsConfig.CORE_BASE_URL + '/user/';
  for (const user of users) {
    await page.evaluate(
      async ({ user, url }) => {
        const options = {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'access-control-allow-origin': '*',
          },
          body: JSON.stringify({
            username: user.username,
            first_name: user.firstName,
            last_name: user.lastName,
            email: user.mail,
            phone: user.phone,
            password: user.password,
          }),
        };
        await fetch(url, options);
      },
      {
        user,
        url,
      }
    );
  }
};
