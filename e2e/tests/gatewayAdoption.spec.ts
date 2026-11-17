import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../config';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart, dockerRestartGateway } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

const GATEWAY_IP_OR_DOMAIN = 'gateway';
const GATEWAY_GRPC_PORT = 50066;

// Parse SSE data lines from a streamed response body.
function parseSseEvents(body: string): Array<Record<string, unknown>> {
  return body
    .split('\n')
    .filter((line) => line.startsWith('data: '))
    .map((line) => JSON.parse(line.slice('data: '.length)));
}

test.describe('Gateway Adoption', () => {
  test.beforeEach(async ({ request }) => {
    // Restore DB to post-wizard snapshot and reset gateway TLS state.
    dockerRestart();
    dockerRestartGateway();

    // Authenticate and create a VPN network for the adoption target.
    const authRes = await request.post('/api/v1/auth', {
      data: { username: defaultUserAdmin.username, password: defaultUserAdmin.password },
    });
    if (!authRes.ok()) throw new Error(`Auth failed: ${authRes.status()}`);

    const networkRes = await request.post('/api/v1/network', {
      data: {
        name: 'test-vpn',
        address: '10.6.0.1/24',
        endpoint: '10.6.0.1',
        port: 51820,
        allowed_ips: '10.6.0.0/24',
        dns: null,
        mtu: 1420,
        fwmark: 0,
        allow_all_groups: true,
        allowed_groups: [],
        keepalive_interval: 25,
        peer_disconnect_threshold: 300,
        acl_enabled: false,
        acl_default_allow: false,
        allowed_ips_from_acl: false,
        mfa_enabled: false,
        posture_checks: [],
        mfa_flows: [],
        service_location_mode: 'disabled',
      },
    });
    if (!networkRes.ok()) {
      throw new Error(
        `Network creation failed: ${networkRes.status()} ${await networkRes.text()}`,
      );
    }
  });

  // Verify that a gateway is registered in the DB with the expected fields after adoption.
  async function assertGatewayPersisted(
    request: Parameters<Parameters<typeof test>[1]>[0]['request'],
    networkId: number,
    expectedName: string,
  ) {
    const statusRes = await request.get(`/api/v1/network/${networkId}/gateways`);
    expect(statusRes.ok()).toBe(true);
    const gateways = await statusRes.json();
    expect(Array.isArray(gateways)).toBe(true);
    expect((gateways as unknown[]).length).toBe(1);

    const gw = (gateways as Array<Record<string, unknown>>)[0];
    expect(gw.name).toBe(expectedName);
    expect(gw.address).toBe(GATEWAY_IP_OR_DOMAIN);
    expect(gw.port).toBe(GATEWAY_GRPC_PORT);
    expect(gw.enabled).toBe(true);
    // certificate_serial being set proves TLS was actually provisioned.
    expect(gw.certificate_serial).toBeTruthy();
  }

  test('SSE endpoint streams adoption steps and ends with Done', async ({ request }) => {
    const networksRes = await request.get('/api/v1/network');
    expect(networksRes.ok()).toBe(true);
    const networks = await networksRes.json();
    expect(networks.length).toBeGreaterThan(0);
    const networkId = (networks[0] as { id: number }).id;

    const sseRes = await request.post(`/api/v1/network/${networkId}/gateways/setup`, {
      data: {
        common_name: 'test-gateway-sse',
        ip_or_domain: GATEWAY_IP_OR_DOMAIN,
        grpc_port: GATEWAY_GRPC_PORT,
      },
      timeout: testsConfig.TEST_TIMEOUT * 1000,
    });
    expect(sseRes.ok()).toBe(true);

    const body = await sseRes.text();
    const events = parseSseEvents(body);

    const steps = events.map((e) => (e as { step: string }).step);
    expect(steps).toContain('CheckingConfiguration');
    expect(steps).toContain('CheckingAvailability');
    expect(steps).toContain('Done');

    const doneEvent = events.find((e) => (e as { step: string }).step === 'Done');
    expect(doneEvent).toBeDefined();
    expect((doneEvent as { error: boolean } | undefined)?.error).toBe(false);

    await assertGatewayPersisted(request, networkId, 'test-gateway-sse');
  });

  test('REST endpoint adopts gateway and returns 201 with gateway data', async ({
    request,
  }) => {
    const networksRes = await request.get('/api/v1/network');
    expect(networksRes.ok()).toBe(true);
    const networks = await networksRes.json();
    expect(networks.length).toBeGreaterThan(0);
    const networkId = (networks[0] as { id: number }).id;

    const adoptRes = await request.post(`/api/v1/network/${networkId}/gateways/adopt`, {
      data: {
        name: 'test-gateway-rest',
        ip_or_domain: GATEWAY_IP_OR_DOMAIN,
        grpc_port: GATEWAY_GRPC_PORT,
      },
      timeout: testsConfig.TEST_TIMEOUT * 1000,
    });
    expect(adoptRes.status()).toBe(201);

    const gateway = await adoptRes.json();
    expect((gateway as { name: string }).name).toBe('test-gateway-rest');

    await assertGatewayPersisted(request, networkId, 'test-gateway-rest');
  });

  test('UI wizard adopts gateway and returns to locations', async ({ page, request }) => {
    const networksRes = await request.get('/api/v1/network');
    expect(networksRes.ok()).toBe(true);
    const networks = await networksRes.json();
    expect(networks.length).toBeGreaterThan(0);
    const networkId = (networks[0] as { id: number }).id;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    await page
      .locator('.virtual-row')
      .filter({ hasText: 'test-vpn' })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Gateway setup', { exact: true }).click();

    await page.getByRole('button', { name: 'Configure gateway' }).click();
    const continueButton = page.getByRole('button', { name: 'Continue' });
    await expect(continueButton).toBeDisabled();
    await page.locator('.checkbox').click();
    await continueButton.click();

    await page.getByTestId('field-common_name').fill('test-gateway-ui');
    await page.getByTestId('field-ip_or_domain').fill(GATEWAY_IP_OR_DOMAIN);
    await page.getByRole('button', { name: 'Adopt Gateway component' }).click();

    await expect(page.locator('.loading-step.variant-success')).toHaveCount(6, {
      timeout: testsConfig.TEST_TIMEOUT * 1000,
    });
    await expect(page.locator('.loading-step.variant-error')).toHaveCount(0);
    await continueButton.click();

    await expect(
      page.getByText('Gateway component configured successfully.'),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Go to locations' }).click();
    await waitForRoute(page, routes.locations);

    await assertGatewayPersisted(request, networkId, 'test-gateway-ui');
  });
});
