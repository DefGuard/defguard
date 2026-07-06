import { request } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../../config';

// Delete all WireGuard networks via the API using an ephemeral admin session.
// Used to reset network state (e.g. remove the dummy network created by
// globalSetup) so the setup wizard is shown again and its UI flow is exercised.
export const apiDeleteAllNetworks = async (): Promise<void> => {
  const apiCtx = await request.newContext({ baseURL: testsConfig.BASE_URL });
  const authRes = await apiCtx.post('/api/v1/auth', {
    data: {
      username: defaultUserAdmin.username,
      password: defaultUserAdmin.password,
    },
  });
  if (!authRes.ok()) throw new Error(`Auth failed: ${authRes.status()}`);
  const listRes = await apiCtx.get('/api/v1/network');
  const networks: { id: number }[] = await listRes.json();
  for (const net of networks) {
    await apiCtx.delete(`/api/v1/network/${net.id}`);
  }
  await apiCtx.dispose();
};
