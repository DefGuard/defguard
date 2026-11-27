import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { loginBasic } from '../utils/controllers/login';
import {
  addPostureOs,
  confirmPostureCheckCreation,
  createPostureCheck,
  drawerValues,
  fillPostureCheckWizard,
  openDrawer,
  openRowMenu,
  postureCell,
  PostureCheckSystem,
  selectPostureOption,
} from '../utils/controllers/postureChecks';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const testNetwork: NetworkForm = {
  name: 'posture location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

const diskEncryption = 'Disk encryption enabled';

const osCases: {
  system: PostureCheckSystem;
  summary: string[];
  row: string;
  drawer: Record<string, string[]>;
}[] = [
  {
    system: {
      os: 'Windows',
      version: 'Windows 11 or higher',
      period: 'Within 60 days',
      conditions: [
        'Connected to Active Directory',
        'Antivirus installed',
        diskEncryption,
      ],
    },
    summary: [
      '11 and higher',
      'Within 60 days',
      'Connected to Active Directory',
      'Antivirus installed',
      diskEncryption,
    ],
    row: '11, Disk encryption, Antivirus, AD joined, Security updates',
    drawer: {
      Version: ['Windows 11 and higher'],
      Other: ['Connected to Active Directory', 'Antivirus installed', diskEncryption],
    },
  },
  {
    system: {
      os: 'macOS',
      version: 'macOS 14 or higher',
      conditions: [diskEncryption, 'Device integrity enabled'],
    },
    summary: ['14 and higher', diskEncryption, 'Device integrity enabled'],
    row: '14, Disk encryption, Device integrity',
    drawer: {
      Version: ['macOS 14 and higher'],
      Other: [diskEncryption, 'Device integrity'],
    },
  },
  {
    system: { os: 'Linux', version: 'Kernel 6 or higher', conditions: [diskEncryption] },
    summary: ['Kernel 6 and higher', diskEncryption],
    row: 'Kernel 6, Disk encryption',
    drawer: { Version: ['Kernel 6 and higher'], Other: [diskEncryption] },
  },
  {
    system: { os: 'iOS', version: 'iOS 18 or higher' },
    summary: ['iOS 18 and higher'],
    row: 'iOS 18+',
    drawer: { Version: ['iOS 18+'] },
  },
  {
    system: { os: 'Android', version: 'Android 14 or higher', period: 'Within 90 days' },
    summary: ['Android 14 and higher'],
    row: 'Android 14+',
    drawer: { Version: ['Android 14+'], 'Security patch': ['Within 90 days'] },
  },
];

test.describe('Posture checks', () => {
  test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Requires DEFGUARD_LICENSE_KEY.');
  test.beforeEach(() => dockerRestart());

  for (const osCase of osCases) {
    test(`Add ${osCase.system.os} posture check`, async ({ page }) => {
      const name = `${osCase.system.os} check`;
      await waitForBase(page);
      await loginBasic(page, defaultUserAdmin);
      await fillPostureCheckWizard(page, { name, systems: [osCase.system] });

      await expect(page.locator('.summary-heading')).toHaveText([
        osCase.system.os,
        'Defguard',
      ]);
      await expect(
        page
          .locator('.summary-row')
          .filter({
            has: page.locator('.summary-heading').getByText(osCase.system.os, {
              exact: true,
            }),
          })
          .locator('.summary-line'),
      ).toHaveText(osCase.summary);
      await confirmPostureCheckCreation(page);

      await page.reload();
      const row = page.locator('.virtual-row').filter({ hasText: name });
      await expect(postureCell(row, osCase.system.os)).toHaveText(osCase.row);
      await openDrawer(page, name);
      for (const [label, values] of Object.entries(osCase.drawer)) {
        await expect(drawerValues(page, osCase.system.os, label)).toHaveText(values);
      }
    });
  }

  test('Client version, pre-release and device attestation are saved', async ({
    page,
  }) => {
    const name = 'client check';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await fillPostureCheckWizard(page, {
      name,
      systems: [{ os: 'Linux' }],
      desktopVersion: '2.1 and higher',
      mobileVersion: '1.7.0 and higher',
      prerelease: true,
      desktopAttestation: true,
      desktopVerification: true,
      mobileAttestation: true,
    });

    await expect(
      page
        .locator('.summary-row')
        .filter({ has: page.locator('.summary-heading').getByText('Defguard') })
        .locator('.summary-line'),
    ).toHaveText([
      'Desktop client: 2.1 and higher',
      'Mobile app: 1.7.0 and higher',
      'Allow pre-release versions of the Defguard client.',
    ]);
    await expect(
      page
        .locator('.summary-row')
        .filter({ has: page.locator('.summary-heading').getByText('Device attestation') })
        .locator('.summary-line'),
    ).toHaveText([
      'Desktop device attestation',
      'Desktop manual device verification',
      'Mobile device attestation',
    ]);
    await confirmPostureCheckCreation(page);

    await page.reload();
    const row = page.locator('.virtual-row').filter({ hasText: name });
    await expect(postureCell(row, 'Desktop client')).toHaveText('Desktop 2.1+');
    await expect(postureCell(row, 'Mobile app')).toHaveText('Defguard 1.7.0+');
    await openDrawer(page, name);
    await expect(drawerValues(page, 'Defguard', 'Desktop')).toHaveText([
      '2.1 and higher',
    ]);
    await expect(drawerValues(page, 'Defguard', 'Mobile app')).toHaveText([
      '1.7.0 and higher',
    ]);
    await expect(drawerValues(page, 'Defguard', 'Other')).toHaveText([
      'Pre-release allowed',
    ]);
    await expect(drawerValues(page, 'Desktop attestation', 'Attestation')).toHaveText([
      'Enabled',
    ]);
    await expect(drawerValues(page, 'Desktop attestation', 'Verification')).toHaveText([
      'Enabled',
    ]);
    await expect(drawerValues(page, 'Mobile attestation', 'Attestation')).toHaveText([
      'Enabled',
    ]);
    await expect(drawerValues(page, 'Mobile attestation', 'Verification')).toHaveText([
      'Disabled',
    ]);
  });

  test('List posture checks', async ({ page, browser }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.postureChecks);
    await expect(page.getByText('No posture checks added yet.')).toBeVisible();

    await createPostureCheck(browser, { name: 'first check', systems: [{ os: 'iOS' }] });
    await createPostureCheck(browser, {
      name: 'second check',
      systems: [{ os: 'Android' }],
    });
    await page.reload();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.filter({ hasText: 'first check' })).toBeVisible();
    await expect(rows.filter({ hasText: 'second check' })).toBeVisible();

    await page.goto(routes.base + routes.activity);
    await expect(
      page
        .locator('#activity-log-page .virtual-row')
        .filter({ hasText: 'Posture check created' }),
    ).toHaveCount(2);
  });

  test('Edit posture check', async ({ page, browser }) => {
    await waitForBase(page);
    await createPostureCheck(browser, {
      name: 'edit check',
      systems: [{ os: 'Linux' }, { os: 'iOS' }],
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.postureChecks);

    await openRowMenu(page, 'edit check', 'Edit');
    await page.waitForURL('**/edit');
    await page.getByRole('textbox', { name: 'Name' }).fill('edited check');
    await page.getByRole('textbox', { name: 'Description' }).fill('edited description');
    await selectPostureOption(
      page,
      page.locator('.policy-os-card', { hasText: 'Linux' }).locator('.select').first(),
      'Kernel 6 or higher',
    );
    await page
      .locator('.policy-os-card', { hasText: 'iOS' })
      .locator('.top .icon-button')
      .click();
    await addPostureOs(page, {
      os: 'macOS',
      version: 'macOS 14 or higher',
      conditions: [diskEncryption],
    });
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Posture check saved')).toBeVisible();

    await page.reload();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(postureCell(rows.first(), 'Title')).toHaveText('edited check');
    await expect(postureCell(rows.first(), 'Linux')).toHaveText('Kernel 6');
    await expect(postureCell(rows.first(), 'macOS')).toHaveText('14, Disk encryption');
    await expect(postureCell(rows.first(), 'iOS')).toHaveText('-');
    const drawer = await openDrawer(page, 'edited check');
    await expect(drawer).toContainText('edited description');
    await expect(drawer.getByText('iOS', { exact: true })).toHaveCount(0);
    await expect(drawerValues(page, 'Linux', 'Version')).toHaveText([
      'Kernel 6 and higher',
    ]);
    await expect(drawerValues(page, 'macOS', 'Version')).toHaveText([
      'macOS 14 and higher',
    ]);
  });

  test('Duplicate posture check', async ({ page, browser }) => {
    await waitForBase(page);
    await createPostureCheck(browser, {
      name: 'original check',
      systems: [{ os: 'Linux', version: 'Kernel 6 or higher' }],
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.postureChecks);

    await openRowMenu(page, 'original check', 'Duplicate');
    await expect(page.getByText('Posture check duplicated')).toBeVisible();
    await page.waitForURL('**/edit');
    await expect(page.getByRole('textbox', { name: 'Name' })).toHaveValue(
      'Copy of original check',
    );

    await page.goto(routes.base + routes.postureChecks);
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.filter({ hasText: 'Copy of original check' })).toContainText(
      'Kernel 6',
    );
  });

  test('Assign posture check to locations', async ({ page, browser }) => {
    await waitForBase(page);
    await createRegularLocation(browser, testNetwork);
    await createPostureCheck(browser, {
      name: 'location check',
      systems: [{ os: 'Linux' }],
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.postureChecks);

    await openRowMenu(page, 'location check', 'Assign to locations');
    const selectionModal = page.locator('#selection-modal');
    await selectionModal
      .locator('.item', { hasText: testNetwork.name })
      .locator('.checkbox')
      .click();
    await selectionModal.getByRole('button', { name: 'Submit', exact: true }).click();
    const confirmModal = page.locator('#action-modal');
    await expect(confirmModal).toContainText(testNetwork.name);
    await confirmModal.getByTestId('confirm-action-submit').click();
    await expect(
      page.getByText('Locations were successfully assigned to the posture check.'),
    ).toBeVisible();

    await page.reload();
    const drawer = await openDrawer(page, 'location check');
    await expect(drawer.locator('.locations-chips')).toHaveText(testNetwork.name);
  });

  test('Delete posture check', async ({ page, browser }) => {
    await waitForBase(page);
    await createPostureCheck(browser, {
      name: 'delete check',
      systems: [{ os: 'Linux' }],
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.postureChecks);

    await openRowMenu(page, 'delete check', 'Delete');
    const modal = page.locator('#action-modal');
    await expect(modal).toContainText('Are you sure you want to delete this check?');
    await modal.getByTestId('confirm-action-submit').click();
    await expect(page.getByText('Posture check deleted')).toBeVisible();

    await page.reload();
    await expect(page.getByText('No posture checks added yet.')).toBeVisible();
  });
});
