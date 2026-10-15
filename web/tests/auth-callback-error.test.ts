import { describe, expect, it } from 'vitest';
import { m } from '../src/paraglide/messages';
import {
  getApiErrorMessage,
  getAuthErrorFromException,
  getAuthErrorText,
} from '../src/shared/api/apiErrorMessages';
import { WebErrorCode } from '../src/shared/api/types';

const invalidClientMsg =
  'Failed to exchange code for ID token; OpenID provider error: Server returned error response: invalid_client: The provided client secret is invalid.';

describe('getAuthErrorFromException', () => {
  it('keeps the backend detail when the OpenID callback fails without an error code', () => {
    expect(
      getAuthErrorFromException({
        response: { status: 401, data: { msg: invalidClientMsg } },
      }),
    ).toEqual({ authErrorDetail: invalidClientMsg });

    expect(
      getAuthErrorFromException({
        response: {
          status: 403,
          data: { msg: 'User not in groups', code: WebErrorCode.UserGroupsNotSynced },
        },
      }),
    ).toEqual({
      authError: WebErrorCode.UserGroupsNotSynced,
      authErrorDetail: 'User not in groups',
    });

    expect(getAuthErrorFromException({ message: 'Network Error' })).toEqual({});
  });
});

describe('getAuthErrorText', () => {
  it('shows a clear message followed by the detail when external sign-in fails', () => {
    expect(getAuthErrorText(undefined, invalidClientMsg)).toEqual([
      m.login_main_external_provider_error(),
      invalidClientMsg,
    ]);

    expect(
      getAuthErrorText(WebErrorCode.UserGroupsNotSynced, 'User not in groups'),
    ).toEqual([
      getApiErrorMessage(WebErrorCode.UserGroupsNotSynced),
      'User not in groups',
    ]);

    expect(getAuthErrorText('not_a_code', 'access_denied')).toEqual([
      m.login_main_external_provider_error(),
      'access_denied',
    ]);

    expect(getAuthErrorText(undefined, undefined)).toEqual([
      m.login_main_external_provider_error(),
    ]);
  });
});
