import { m } from '../../paraglide/messages';
import type { ApiErrorMessageKey, WebErrorCode } from './types';

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
