import { APIRequestContext, expect } from '@playwright/test';

import { testsConfig } from '../../config';

type MailpitSummary = {
  ID: string;
  Subject: string;
};

type MailpitSearchResponse = {
  messages: MailpitSummary[];
};

type MailpitMessage = {
  Subject: string;
  Text: string;
};

const url = (path: string) => testsConfig.MAILPIT_URL + '/api/v1' + path;

export const apiClearMailbox = async (request: APIRequestContext): Promise<void> => {
  const response = await request.delete(url('/messages'));
  expect(response.ok()).toBe(true);
};

const apiSearchMail = async (
  request: APIRequestContext,
  to: string,
  subject?: string,
): Promise<MailpitSummary[]> => {
  const query = subject ? `to:"${to}" subject:"${subject}"` : `to:"${to}"`;
  const response = await request.get(url('/search'), { params: { query } });
  expect(response.ok()).toBe(true);
  const { messages }: MailpitSearchResponse = await response.json();
  return messages;
};

export const apiWaitForMail = async (
  request: APIRequestContext,
  to: string,
  subject?: string,
): Promise<MailpitMessage> => {
  await expect
    .poll(async () => (await apiSearchMail(request, to, subject)).length, {
      message: `No email ${subject ?? ''} delivered to ${to}`,
      timeout: testsConfig.TEST_TIMEOUT * 1000,
    })
    .toBeGreaterThan(0);
  const [latest] = await apiSearchMail(request, to, subject);
  const response = await request.get(url(`/message/${latest.ID}`));
  expect(response.ok()).toBe(true);
  return response.json();
};
