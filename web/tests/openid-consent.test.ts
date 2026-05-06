import { describe, expect, it } from 'vitest';
import { searchSchema } from '../src/routes/consent';

// Regression tests for DG26-7: OAuth state parameter parsing (RFC 6749 Appendix A.5).
//
// TanStack Router parses URL search params with JSON-like type inference, so a
// purely numeric state value (e.g. state=123456) arrives at Zod as a JS number.
// The schema must coerce it to a string instead of rejecting it.
// State is also optional per RFC 6749 - it must not be required by the schema.

const validBase = {
  client_id: 'dFeyrDTcUqvzYcTY',
  redirect_uri: 'https://example.com/callback',
  response_type: 'code',
  scope: 'openid profile',
};

describe('consent route searchSchema - state parameter', () => {
  it('accepts a normal string state', () => {
    const result = searchSchema.safeParse({ ...validBase, state: 'ABCDEF' });
    expect(result.success).toBe(true);
    expect(result.data?.state).toBe('ABCDEF');
  });

  it('coerces a numeric state to string (DG26-7 regression)', () => {
    // TanStack Router hands Zod a number when the query param looks numeric.
    const result = searchSchema.safeParse({ ...validBase, state: 123456 });
    expect(result.success).toBe(true);
    expect(result.data?.state).toBe('123456');
  });

  it('accepts state absent entirely (state is optional per RFC 6749)', () => {
    const result = searchSchema.safeParse({ ...validBase });
    expect(result.success).toBe(true);
    expect(result.data?.state).toBeUndefined();
  });

  it('rejects an empty string state (min length 1)', () => {
    const result = searchSchema.safeParse({ ...validBase, state: '' });
    expect(result.success).toBe(false);
  });
});
