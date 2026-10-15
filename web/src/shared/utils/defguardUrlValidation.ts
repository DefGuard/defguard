import type z from 'zod';
import type { ExternalSslType, InternalSslType } from '../api/types';
import { getCorrectedUrl } from './defguardUrl';

type UrlValue = Record<string, unknown> & {
  ssl_type: string;
};

export const createDistinctUrlRefinement = (
  getOtherUrl: () => string | null | undefined,
  field: string,
  message: string,
) => {
  return (value: UrlValue, ctx: z.RefinementCtx) => {
    const otherUrl = getOtherUrl();
    if (!otherUrl) return;

    const correctedUrl = getCorrectedUrl(
      value[field] as string,
      value.ssl_type as InternalSslType | ExternalSslType,
    );
    const normalizeUrl = (url: string) => {
      try {
        return new URL(url).toString();
      } catch {
        return url;
      }
    };

    if (normalizeUrl(correctedUrl) === normalizeUrl(otherUrl)) {
      ctx.addIssue({ code: 'custom', path: [field], message });
    }
  };
};
