import { describe, expect, it, vi } from 'vitest';
import { getDeletePostureCheckModalData } from '../src/pages/PostureChecksPage/postureChecks';
import api from '../src/shared/api/api';

describe('posture check delete confirmation', () => {
  it('builds a destructive confirmation modal for deleting a posture check', async () => {
    const deleteDevicePosture = vi
      .spyOn(api.devicePosture, 'deleteDevicePosture')
      .mockResolvedValue(undefined as never);

    const modalData = getDeletePostureCheckModalData(
      {
        id: 42,
        name: 'First posture check',
      },
      ['Warsaw', 'Berlin'],
    );

    expect(modalData.title).toBe('Delete posture check');
    expect(modalData.contentMd).toBe(
      'Are you sure you want to delete this check? It’s currently used in Warsaw and Berlin. Removing it may change access criteria for users in these locations.',
    );
    expect(modalData.invalidateKeys).toEqual([['device-posture'], ['network']]);
    expect(modalData.submitProps).toEqual({
      text: 'Delete',
      variant: 'critical',
    });

    await modalData.actionPromise();

    expect(deleteDevicePosture).toHaveBeenCalledWith(42);
  });

  it('formats three or more assigned locations in the confirmation copy', () => {
    const modalData = getDeletePostureCheckModalData(
      {
        id: 42,
        name: 'First posture check',
      },
      ['Warsaw', 'Berlin', 'Paris'],
    );

    expect(modalData.contentMd).toBe(
      'Are you sure you want to delete this check? It’s currently used in Warsaw, Berlin, and Paris. Removing it may change access criteria for users in these locations.',
    );
  });

  it('omits the location warning when the check is not assigned anywhere', () => {
    const modalData = getDeletePostureCheckModalData(
      {
        id: 42,
        name: 'First posture check',
      },
      [],
    );

    expect(modalData.contentMd).toBe('Are you sure you want to delete this check?');
  });
});
