import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AddOpenIdProvider,
  DirectorySyncBehavior,
  type DirectorySyncBehaviorValue,
  DirectorySyncTarget,
  type DirectorySyncTargetValue,
  type TestDirectorySyncResponse,
} from '../../shared/api/types';
import type { SelectOption } from '../../shared/defguard-ui/components/Select/types';

export const validateExternalProviderWizard = async (
  values: AddOpenIdProvider,
): Promise<TestDirectorySyncResponse | boolean> => {
  try {
    await api.openIdProvider.addOpenIdProvider(values);
    if (values.directory_sync_enabled) {
      const { data: result } = await api.openIdProvider.testDirectorySync();
      return result;
    }
  } catch (_) {
    return false;
  }
  return true;
};

export const directorySyncBehaviorName: Record<DirectorySyncBehaviorValue, string> = {
  delete: m.controls_delete(),
  disable: m.controls_disable(),
  keep: m.controls_keep(),
};

export const directorySyncTargetName: Record<DirectorySyncTargetValue, string> = {
  all: m.cmp_sync_behavior_target_all(),
  groups: m.cmp_sync_behavior_target_groups(),
  users: m.cmp_sync_behavior_target_users(),
};

export const directorySyncBehaviorOptions: SelectOption<DirectorySyncBehaviorValue>[] = [
  {
    key: DirectorySyncBehavior.Keep,
    value: DirectorySyncBehavior.Keep,
    label: directorySyncBehaviorName[DirectorySyncBehavior.Keep],
  },
  {
    label: directorySyncBehaviorName[DirectorySyncBehavior.Disable],
    key: DirectorySyncBehavior.Disable,
    value: DirectorySyncBehavior.Disable,
  },
  {
    key: DirectorySyncBehavior.Delete,
    value: DirectorySyncBehavior.Delete,
    label: directorySyncBehaviorName[DirectorySyncBehavior.Delete],
  },
];

export const directorySyncTargetOptions: SelectOption<DirectorySyncTargetValue>[] = [
  {
    key: DirectorySyncTarget.All,
    value: DirectorySyncTarget.All,
    label: directorySyncTargetName[DirectorySyncTarget.All],
  },
  {
    key: DirectorySyncTarget.Users,
    value: DirectorySyncTarget.Users,
    label: directorySyncTargetName[DirectorySyncTarget.Users],
  },
  {
    key: DirectorySyncTarget.Groups,
    value: DirectorySyncTarget.Groups,
    label: directorySyncTargetName[DirectorySyncTarget.Groups],
  },
];
