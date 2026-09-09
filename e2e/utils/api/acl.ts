import { expect, Page } from '@playwright/test';

import { testsConfig } from '../../config';

type AclState = 'New' | 'Modified' | 'Applied' | 'Deleted' | 'Expired';

export type ApiAclAlias = {
  id: number;
  parent_id: number | null;
  name: string;
  state: AclState;
  addresses: string;
  ports: string;
  protocols: number[];
  rules: number[];
};

export type ApiAclDestination = ApiAclAlias & {
  any_address: boolean;
  any_port: boolean;
  any_protocol: boolean;
};

export type ApiAclRule = {
  id: number;
  parent_id: number | null;
  name: string;
  state: AclState;
  enabled: boolean;
};

const url = (path: string) => testsConfig.CORE_BASE_URL + path;

const post = async (page: Page, path: string, data: unknown): Promise<number> => {
  const response = await page.request.post(url(path), {
    data,
    headers: { 'Content-Type': 'application/json' },
  });
  expect(response.status()).toBe(201);
  const body = await response.json();
  return body.id;
};

const put = async (page: Page, path: string, data: unknown): Promise<void> => {
  const response = await page.request.put(url(path), {
    data,
    headers: { 'Content-Type': 'application/json' },
  });
  expect(response.ok()).toBe(true);
};

const get = async <T>(page: Page, path: string): Promise<T> => {
  const response = await page.request.get(url(path), {
    headers: { 'Content-Type': 'application/json' },
  });
  expect(response.ok()).toBe(true);
  return response.json();
};

const aliasPayload = (name: string) => ({
  name,
  addresses: '10.10.0.0/24',
  ports: '443',
  protocols: [6],
});

const destinationPayload = (name: string) => ({
  name,
  addresses: '10.20.0.0/24',
  ports: '443',
  protocols: [6],
  any_address: false,
  any_port: false,
  any_protocol: false,
});

const rulePayload = (name: string, overrides: Record<string, unknown> = {}) => ({
  name,
  all_locations: false,
  locations: [],
  expires: null,
  enabled: true,
  allow_all_users: true,
  deny_all_users: false,
  allow_all_groups: false,
  deny_all_groups: false,
  allow_all_network_devices: false,
  deny_all_network_devices: false,
  allowed_users: [],
  denied_users: [],
  allowed_groups: [],
  denied_groups: [],
  allowed_network_devices: [],
  denied_network_devices: [],
  addresses: '',
  ports: '',
  protocols: [],
  aliases: [],
  destinations: [],
  any_address: true,
  any_port: true,
  any_protocol: true,
  use_manual_destination_settings: true,
  ...overrides,
});

export const apiCreateAlias = (page: Page, name: string): Promise<number> =>
  post(page, '/acl/alias', aliasPayload(name));

export const apiCreateDestination = (page: Page, name: string): Promise<number> =>
  post(page, '/acl/destination', destinationPayload(name));

export const apiCreateRule = (
  page: Page,
  name: string,
  overrides: Record<string, unknown> = {},
): Promise<number> => post(page, '/acl/rule', rulePayload(name, overrides));

export const apiModifyAlias = (page: Page, id: number, name: string): Promise<void> =>
  put(page, `/acl/alias/${id}`, aliasPayload(name));

export const apiModifyDestination = (
  page: Page,
  id: number,
  name: string,
): Promise<void> => put(page, `/acl/destination/${id}`, destinationPayload(name));

export const apiApplyRules = (page: Page, rules: number[]): Promise<void> =>
  put(page, '/acl/rule/apply', { rules });

export const apiGetAliases = (page: Page): Promise<ApiAclAlias[]> =>
  get(page, '/acl/alias');

export const apiGetDestinations = (page: Page): Promise<ApiAclDestination[]> =>
  get(page, '/acl/destination');

export const apiGetRules = async (page: Page): Promise<ApiAclRule[]> => {
  const body = await get<{ data: ApiAclRule[] }>(page, '/acl/rule');
  return body.data;
};
