import { QueryClient } from '@tanstack/react-query';
import { describe, expect, it } from 'vitest';
import {
  getMfaFlowsQueryOptions,
  getMfaMethodAvailabilityQueryOptions,
  mfaAvailabilityInvalidateKey,
} from '../src/shared/query';

describe('MFA availability invalidation', () => {
  it('marks both MFA queries stale so the editor reflects a new license, SMTP or OpenID provider', async () => {
    const client = new QueryClient();
    client.setQueryData(getMfaFlowsQueryOptions.queryKey, []);
    client.setQueryData(getMfaMethodAvailabilityQueryOptions.queryKey, []);

    await client.invalidateQueries({ queryKey: mfaAvailabilityInvalidateKey });

    expect(client.getQueryState(getMfaFlowsQueryOptions.queryKey)?.isInvalidated).toBe(
      true,
    );
    expect(
      client.getQueryState(getMfaMethodAvailabilityQueryOptions.queryKey)?.isInvalidated,
    ).toBe(true);
  });
});
