import { m } from '../../paraglide/messages';
import type { ApiErrorMessageKey, WebErrorCode } from './types';

export function getApiErrorMessage(code: WebErrorCode): string {
  const key: ApiErrorMessageKey = `api_error_${code}`;
  return (m as Record<ApiErrorMessageKey, () => string>)[key]();
}
