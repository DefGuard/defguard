import type { AxiosError } from 'axios';
import { m } from '../../paraglide/messages';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { type ApiError, type ApiErrorMessageKey, WebErrorCode } from './types';

const isWebErrorCode = (value: string): value is WebErrorCode =>
  (Object.values(WebErrorCode) as string[]).includes(value);

export function getAuthErrorFromException(e: unknown): {
  authError?: WebErrorCode;
  authErrorDetail?: string;
} {
  const data = (e as AxiosError<ApiError>).response?.data;
  return { authError: data?.code, authErrorDetail: data?.msg };
}

export function getApiErrorMessage(code: WebErrorCode, defaultMessage?: string): string {
  const key: ApiErrorMessageKey = `api_error_${code}`;
  const messageFn = (m as Partial<Record<ApiErrorMessageKey, () => string>>)[key];
  if (messageFn) {
    return messageFn();
  }
  if (defaultMessage) {
    return defaultMessage;
  }
  return m.error_unknown();
}

export function getAuthErrorText(code?: string, detail?: string): string[] {
  const message =
    isPresent(code) && isWebErrorCode(code)
      ? getApiErrorMessage(code)
      : m.login_main_external_provider_error();
  return isPresent(detail) ? [message, detail] : [message];
}
