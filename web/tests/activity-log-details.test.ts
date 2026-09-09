import { describe, expect, it } from 'vitest';
import {
  buildLogDetailsChanges,
  hasLogDetails,
} from '../src/pages/ActivityLogPage/logDetails';
import { ActivityLogEventType } from '../src/shared/api/activity-log-types';

describe('activity log details', () => {
  it('marks settings events as clickable and leaves other events alone', () => {
    expect(hasLogDetails(ActivityLogEventType.SettingsUpdatedPartial)).toBe(true);
    expect(hasLogDetails(ActivityLogEventType.EnterpriseSettingsUpdated)).toBe(true);
    expect(hasLogDetails(ActivityLogEventType.UserLogin)).toBe(false);
  });

  it('builds labelled rows for changed and unchanged settings fields', () => {
    const changes = buildLogDetailsChanges({
      before: {
        smtp_server: 'test.email.com',
        smtp_sender: 'no-reply@mailing.example',
        smtp_port: 587,
      },
      after: {
        smtp_server: 'new_test.email.com',
        smtp_sender: 'no-reply@defguard.net',
        smtp_port: 587,
      },
    });

    expect(changes).toEqual([
      {
        field: 'smtp_server',
        label: 'Server address',
        from: 'test.email.com',
        to: 'new_test.email.com',
        changed: true,
      },
      {
        field: 'smtp_sender',
        label: 'Sender email address',
        from: 'no-reply@mailing.example',
        to: 'no-reply@defguard.net',
        changed: true,
      },
      {
        field: 'smtp_port',
        label: 'Server port',
        from: '587',
        to: '587',
        changed: false,
      },
    ]);
  });

  it('formats booleans, empty values and lists', () => {
    const changes = buildLogDetailsChanges({
      before: {
        ldap_enabled: false,
        ldap_url: null,
        ldap_sync_groups: [],
      },
      after: {
        ldap_enabled: true,
        ldap_url: 'ldap://127.0.0.1',
        ldap_sync_groups: ['admins', 'users'],
      },
    });

    expect(changes).toEqual([
      {
        field: 'ldap_enabled',
        label: 'LDAP integration',
        from: 'Disabled',
        to: 'Enabled',
        changed: true,
      },
      {
        field: 'ldap_url',
        label: 'LDAP URL',
        from: '—',
        to: 'ldap://127.0.0.1',
        changed: true,
      },
      {
        field: 'ldap_sync_groups',
        label: 'Synchronized groups',
        from: '—',
        to: 'admins, users',
        changed: true,
      },
    ]);
  });

  it('falls back to a humanized label for fields without a message', () => {
    const changes = buildLogDetailsChanges({
      before: { some_new_setting: 'a', instance_name: 'Old' },
      after: { some_new_setting: 'b', instance_name: 'New' },
    });

    expect(changes).toEqual([
      {
        field: 'some_new_setting',
        label: 'Some new setting',
        from: 'a',
        to: 'b',
        changed: true,
      },
      {
        field: 'instance_name',
        label: 'Instance name',
        from: 'Old',
        to: 'New',
        changed: true,
      },
    ]);
  });

  it('returns no rows when the event carries no before/after metadata', () => {
    expect(buildLogDetailsChanges(null)).toEqual([]);
    expect(buildLogDetailsChanges({ stream: { id: 1 } })).toEqual([]);
  });
});
