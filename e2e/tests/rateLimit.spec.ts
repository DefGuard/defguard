import {
  APIRequestContext,
  APIResponse,
  Browser,
  expect,
  Page,
  test,
} from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import {
  apiCreateDevice,
  apiCreateMfaLocation,
  ClientDevice,
  clientMfaConnect,
  clientMfaFinish,
  clientMfaStart,
  clientMfaStepStart,
  ClientProtocol,
  MfaMethod,
  presharedKey,
  waitForProxy,
} from '../utils/api/clientMfa';
import { apiLogin } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { setupSMTP } from '../utils/controllers/settings';
import { endThrottleWindows } from '../utils/db/endThrottleWindows';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const LOGIN_LIMIT = 5;
const SESSION_FAILED_ATTEMPT_CAP = 5;
const VPN_CODE_LIMIT = 10;
const VPN_INITIATE_LIMIT = 20;

const WRONG_CODE = '000000';
const EMAIL_CODE_PERIOD = 300;

const mfaCode = async (secret: string, method: MfaMethod): Promise<string> => {
  const options =
    method === MfaMethod.EMAIL ? { digits: 6, period: EMAIL_CODE_PERIOD } : undefined;
  return (await TOTP.generate(secret, options)).otp;
};

const expectPreconditionError = async (response: APIResponse, message: string) => {
  expect(response.status()).toBe(428);
  expect((await response.json()).error).toBe(message);
};

const expectConnected = async (response: APIResponse) => {
  expect(response.status()).toBe(200);
  expect(await presharedKey(response)).toBeTruthy();
};

const countActivityEvents = async (
  api: APIRequestContext,
  event: string,
  username: string,
): Promise<number> => {
  const response = await api.get(testsConfig.CORE_BASE_URL + '/activity_log', {
    params: { event, username },
  });
  expect(response.ok()).toBe(true);
  return (await response.json()).pagination.total_items;
};

test.describe('Rate limiting', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'testuser' };
  });

  test('Failed password logins lock only that account until the window ends', async ({
    page,
    browser,
    request,
  }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const login = (username: string, password: string) =>
      apiLogin(request, username, password);

    // A correct password refunds its attempt.
    for (let i = 0; i < LOGIN_LIMIT - 1; i++) {
      expect(await login(testUser.username, 'wrong')).toBe(401);
    }
    expect(await login(testUser.username, testUser.password)).toBe(200);
    expect(await login(testUser.username, 'wrong')).toBe(401);
    expect(await login(testUser.username, 'wrong')).toBe(429);

    expect(await login(testUser.mail, testUser.password)).toBe(429);
    expect(await login(testUser.username, testUser.password)).toBe(429);
    expect(await login(defaultUserAdmin.username, defaultUserAdmin.password)).toBe(200);

    for (let i = 0; i < LOGIN_LIMIT; i++) {
      expect(await login('unknownuser', 'wrong')).toBe(401);
    }
    expect(await login('unknownuser', 'wrong')).toBe(429);

    await endThrottleWindows();
    expect(await login(testUser.username, testUser.password)).toBe(200);
  });

  for (const { name, enable, verifyPath, method } of [
    {
      name: 'TOTP',
      enable: enableTOTP,
      verifyPath: '/auth/totp/verify',
      method: MfaMethod.TOTP,
    },
    {
      name: 'email',
      enable: enableEmailMFA,
      verifyPath: '/auth/email/verify',
      method: MfaMethod.EMAIL,
    },
  ]) {
    test(`Failed ${name} codes lock the web login and its password step until the window ends`, async ({
      page,
      browser,
      request,
    }) => {
      await waitForBase(page);
      await createUser(browser, testUser);
      const { secret } = await enable(browser, testUser);
      const login = () => apiLogin(request, testUser.username, testUser.password);
      const verify = async (code: string) =>
        (
          await request.post(testsConfig.CORE_BASE_URL + verifyPath, { data: { code } })
        ).status();

      expect(await login()).toBe(201);
      for (let i = 0; i < LOGIN_LIMIT; i++) {
        expect(await verify(WRONG_CODE)).toBe(401);
      }
      expect(await verify(await mfaCode(secret, method))).toBe(429);
      expect(await login()).toBe(429);

      await endThrottleWindows();
      expect(await login()).toBe(201);
      expect(await verify(await mfaCode(secret, method))).toBe(200);
    });
  }

  const seedVpnUser = async (
    page: Page,
    browser: Browser,
    request: APIRequestContext,
    method: MfaMethod,
  ) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    let secret: string;
    if (method === MfaMethod.EMAIL) {
      ({ secret } = await enableEmailMFA(browser, testUser));
    } else {
      await setupSMTP(browser);
      ({ secret } = await enableTOTP(browser, testUser));
    }
    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await waitForProxy(request);
    const locationId = await apiCreateMfaLocation(request);
    const pubkeys = [
      await apiCreateDevice(request, testUser.username, 'first-device'),
      await apiCreateDevice(request, testUser.username, 'second-device'),
    ];
    return { secret, locationId, pubkeys };
  };

  const codeLimitCases: { protocol: ClientProtocol; method: MfaMethod }[] = [
    { protocol: 'multi-step', method: MfaMethod.TOTP },
    { protocol: 'legacy', method: MfaMethod.TOTP },
    { protocol: 'multi-step', method: MfaMethod.EMAIL },
  ];
  for (const { protocol, method } of codeLimitCases) {
    test(`VPN client MFA failed ${MfaMethod[method]} codes lock only that device across sessions (${protocol} client)`, async ({
      page,
      browser,
      request,
    }) => {
      const { secret, locationId, pubkeys } = await seedVpnUser(
        page,
        browser,
        request,
        method,
      );
      const [first, second]: ClientDevice[] = pubkeys.map((pubkey) => ({
        locationId,
        pubkey,
        protocol,
        method,
      }));
      // A legacy client sends no attempt id, so Core cannot tell it why a code is refused.
      const exhausted = protocol === 'legacy' ? 401 : 403;

      // The failed-code count survives a new session, and each session aborts at its own cap.
      let attempt = await clientMfaConnect(request, first);
      for (let i = 1; i <= SESSION_FAILED_ATTEMPT_CAP; i++) {
        const response = await clientMfaFinish(request, attempt, WRONG_CODE);
        expect(response.status()).toBe(
          i === SESSION_FAILED_ATTEMPT_CAP ? exhausted : 401,
        );
      }
      for (const wrongCodes of [VPN_CODE_LIMIT - SESSION_FAILED_ATTEMPT_CAP - 1, 1]) {
        attempt = await clientMfaConnect(request, first);
        for (let i = 0; i < wrongCodes; i++) {
          expect((await clientMfaFinish(request, attempt, WRONG_CODE)).status()).toBe(
            401,
          );
        }
      }

      const blocked = await clientMfaFinish(
        request,
        attempt,
        await mfaCode(secret, method),
      );
      expect(blocked.status()).toBe(exhausted);
      await expectPreconditionError(
        await clientMfaStart(request, first),
        'Too many failed MFA attempts. Try again later.',
      );

      const secondAttempt = await clientMfaConnect(request, second);
      await expectConnected(
        await clientMfaFinish(request, secondAttempt, await mfaCode(secret, method)),
      );

      await endThrottleWindows();
      attempt = await clientMfaConnect(request, first);
      await expectConnected(
        await clientMfaFinish(request, attempt, await mfaCode(secret, method)),
      );

      // The refused valid code is not verified, so it adds no failed event.
      await expect
        .poll(() =>
          countActivityEvents(request, 'vpn_client_mfa_failed', testUser.username),
        )
        .toBe(VPN_CODE_LIMIT);
    });
  }

  test('Repeated VPN client MFA step starts lock only that device', async ({
    page,
    browser,
    request,
  }) => {
    const { secret, locationId, pubkeys } = await seedVpnUser(
      page,
      browser,
      request,
      MfaMethod.TOTP,
    );
    const [first, second]: ClientDevice[] = pubkeys.map((pubkey) => ({
      locationId,
      pubkey,
      protocol: 'multi-step',
      method: MfaMethod.TOTP,
    }));

    // The flow start initiates the first step, so it uses one request of the limit.
    let attempt = await clientMfaConnect(request, first);
    for (let i = 1; i < VPN_INITIATE_LIMIT; i++) {
      expect(
        (await clientMfaStepStart(request, attempt.token, first.method)).status(),
      ).toBe(200);
    }

    await expectPreconditionError(
      await clientMfaStepStart(request, attempt.token, first.method),
      'Too many MFA requests. Try again later.',
    );
    await expectPreconditionError(
      await clientMfaStart(request, first),
      'Too many MFA requests. Try again later.',
    );

    const secondAttempt = await clientMfaConnect(request, second);
    await expectConnected(
      await clientMfaFinish(
        request,
        secondAttempt,
        await mfaCode(secret, MfaMethod.TOTP),
      ),
    );

    await endThrottleWindows();
    attempt = await clientMfaConnect(request, first);
    await expectConnected(
      await clientMfaFinish(request, attempt, await mfaCode(secret, MfaMethod.TOTP)),
    );
  });
});
