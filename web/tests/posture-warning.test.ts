import { describe, expect, it, vi } from 'vitest';
import { openModal } from '../src/shared/hooks/modalControls/modalsSubjects';

vi.mock('../src/shared/hooks/modalControls/modalsSubjects', () => ({
  openModal: vi.fn(),
}));

vi.mock('../src/shared/hooks/modalControls/modalTypes', () => ({
  ModalName: { ConfirmAction: 'confirmAction' },
}));

vi.mock('../src/paraglide/messages', () => ({
  m: {
    modal_posture_assignment_warning_added: () => 'Added:',
    modal_posture_assignment_warning_removed: () => 'Removed:',
    modal_posture_assignment_warning_title: () => 'Confirm posture check changes',
    modal_posture_assignment_warning_body_location: ({ changes }: { changes: string }) =>
      `These changes may disconnect active VPN client sessions for this location.\n\n${changes}`,
    modal_posture_assignment_warning_body_postures: ({ changes }: { changes: string }) =>
      `These changes may disconnect active VPN client sessions for the affected locations.\n\n${changes}`,
    modal_posture_rules_warning_body: () =>
      'These rules are enforced the next time each device connects. Devices connected now are unaffected until they reconnect.',
    controls_save_changes_anyway: () => 'Save changes anyway',
  },
}));

import {
  confirmLocationSelectionChange,
  confirmPostureSelectionChange,
} from '../src/shared/utils/postureWarning';

type Option = { readonly id: number; readonly label: string };

const locOptions: Option[] = [
  { id: 1, label: 'Berlin' },
  { id: 2, label: 'Amsterdam' },
  { id: 3, label: 'Zurich' },
];

const noop = async () => {};

afterEach(() => {
  vi.clearAllMocks();
});

describe('confirmPostureSelectionChange', () => {
  it('returns false and opens nothing when the id sets are identical', () => {
    const result = confirmPostureSelectionChange({
      current: [1, 2],
      next: [1, 2],
      options: locOptions,
      actionPromise: noop,
    });

    expect(result).toBe(false);
    expect(openModal).not.toHaveBeenCalled();
  });

  it('returns false and opens nothing when both sets are empty', () => {
    const result = confirmPostureSelectionChange({
      current: [],
      next: [],
      options: locOptions,
      actionPromise: noop,
    });

    expect(result).toBe(false);
    expect(openModal).not.toHaveBeenCalled();
  });

  it('opens a modal with the Added group when items were added', () => {
    confirmPostureSelectionChange({
      current: [1],
      next: [1, 2, 3],
      options: locOptions,
      actionPromise: noop,
    });

    expect(openModal).toHaveBeenCalledOnce();
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    // Added: group sorted alphabetically
    expect(contentMd).toContain('**Added:**');
    expect(contentMd).toContain('- Amsterdam');
    expect(contentMd).toContain('- Zurich');
    // Added: items appear in alphabetical order
    expect(contentMd.indexOf('Amsterdam')).toBeLessThan(contentMd.indexOf('Zurich'));
    expect(contentMd).not.toContain('Removed');
  });

  it('opens a modal with the Removed group when items were removed', () => {
    confirmPostureSelectionChange({
      current: [1, 2, 3],
      next: [1],
      options: locOptions,
      actionPromise: noop,
    });

    expect(openModal).toHaveBeenCalledOnce();
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('**Removed:**');
    expect(contentMd).toContain('- Amsterdam');
    expect(contentMd).toContain('- Zurich');
    expect(contentMd.indexOf('Amsterdam')).toBeLessThan(contentMd.indexOf('Zurich'));
    expect(contentMd).not.toContain('Added');
  });

  it('opens a modal with both Added and Removed groups when items changed', () => {
    confirmPostureSelectionChange({
      current: [1],
      next: [2, 3],
      options: locOptions,
      actionPromise: noop,
    });

    expect(openModal).toHaveBeenCalledOnce();
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('**Added:**');
    expect(contentMd).toContain('**Removed:**');
    expect(contentMd.indexOf('Added:')).toBeLessThan(contentMd.indexOf('Removed:'));
  });

  it('includes the location-warning body message', () => {
    confirmPostureSelectionChange({
      current: [1],
      next: [2],
      options: locOptions,
      actionPromise: noop,
    });

    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('for this location.');
  });

  it('accepts Sets as well as arrays', () => {
    const result = confirmPostureSelectionChange({
      current: new Set([1, 2]),
      next: new Set([2, 3]),
      options: locOptions,
      actionPromise: noop,
    });

    expect(result).toBe(true);
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('**Added:**');
    expect(contentMd).toContain('- Zurich');
    expect(contentMd).toContain('**Removed:**');
    expect(contentMd).toContain('- Berlin');
  });
});

describe('confirmLocationSelectionChange', () => {
  it('returns false when no diff and no deferredEnforcement', () => {
    const result = confirmLocationSelectionChange({
      current: [1, 2],
      next: [1, 2],
      options: locOptions,
      actionPromise: noop,
    });

    expect(result).toBe(false);
    expect(openModal).not.toHaveBeenCalled();
  });

  it('includes the postures-warning body message', () => {
    confirmLocationSelectionChange({
      current: [1],
      next: [2],
      options: locOptions,
      actionPromise: noop,
    });

    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('for the affected locations.');
  });

  it('opens rules-only body when deferredEnforcement is true and no location diff', () => {
    confirmLocationSelectionChange({
      current: [1, 2],
      next: [1, 2],
      options: locOptions,
      actionPromise: noop,
      deferredEnforcement: true,
    });

    expect(openModal).toHaveBeenCalledOnce();
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('These rules are enforced');
    expect(contentMd).not.toContain('Added');
    expect(contentMd).not.toContain('Removed');
  });

  it('appends rules paragraph after locations diff when deferredEnforcement is true and diff exists', () => {
    confirmLocationSelectionChange({
      current: [1],
      next: [2],
      options: locOptions,
      actionPromise: noop,
      deferredEnforcement: true,
    });

    expect(openModal).toHaveBeenCalledOnce();
    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('for the affected locations.');
    expect(contentMd).toContain('These rules are enforced');
    // rules paragraph appears after the locations body
    const locIndex = contentMd.indexOf('for the affected locations.');
    const rulesIndex = contentMd.indexOf('These rules are enforced');
    expect(locIndex).toBeLessThan(rulesIndex);
  });

  it('returns false when deferredEnforcement is false (the falsy default)', () => {
    const result = confirmLocationSelectionChange({
      current: [1],
      next: [1],
      options: locOptions,
      actionPromise: noop,
      deferredEnforcement: false,
    });

    expect(result).toBe(false);
  });
});

describe('markdown escaping', () => {
  it('escapes markdown control characters in labels', () => {
    const spikyOptions: Option[] = [
      { id: 1, label: 'plain' },
      { id: 2, label: '*bold*' },
      { id: 3, label: '[link](url)' },
      { id: 4, label: 'back`tick' },
    ];

    confirmPostureSelectionChange({
      current: [1],
      next: [2, 3, 4],
      options: spikyOptions,
      actionPromise: noop,
    });

    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    // Asterisks escaped
    expect(contentMd).toContain('\\*bold\\*');
    expect(contentMd).not.toMatch(/(?<!\\)\*bold\*/);
    // Brackets/parens escaped
    expect(contentMd).toContain('\\[link\\]\\(url\\)');
    // Backtick escaped
    expect(contentMd).toContain('back\\`tick');
    // Plain label untouched
    expect(contentMd).toContain('- plain');
  });

  it('prevents underscores from being interpreted as italics', () => {
    const underscoreOptions: Option[] = [
      { id: 1, label: 'old' },
      { id: 2, label: 'os_version_check' },
    ];

    confirmPostureSelectionChange({
      current: [1],
      next: [2],
      options: underscoreOptions,
      actionPromise: noop,
    });

    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).not.toMatch(/(?<!\\)_os_version_check/);
  });
});

describe('unknown id fallback', () => {
  it('uses String(id) when an id has no matching option', () => {
    confirmPostureSelectionChange({
      current: [],
      next: [1, 99],
      options: [{ id: 1, label: 'Known' }],
      actionPromise: noop,
    });

    const contentMd = vi.mocked(openModal).mock.calls[0][1].contentMd;
    expect(contentMd).toContain('- 99');
    expect(contentMd).toContain('- Known');
  });
});

describe('modal structure', () => {
  it('opens ConfirmAction with the shared title, critical variant, and actionPromise', () => {
    const actionPromise = async () => 'saved';
    confirmPostureSelectionChange({
      current: [1],
      next: [2],
      options: locOptions,
      actionPromise,
    });

    const modalData = vi.mocked(openModal).mock.calls[0][1];
    expect(openModal).toHaveBeenCalledWith(
      'confirmAction',
      expect.objectContaining({
        title: 'Confirm posture check changes',
        actionPromise,
        submitProps: {
          text: 'Save changes anyway',
          variant: 'critical',
        },
      }),
    );
    // No stale keys from the old helper
    expect(modalData.invalidateKeys).toBeUndefined();
    expect(modalData.onSuccess).toBeUndefined();
    expect(modalData.onError).toBeUndefined();
  });
});
