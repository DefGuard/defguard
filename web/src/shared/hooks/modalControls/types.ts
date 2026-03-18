import type { QueryKey } from '@tanstack/react-query';
import type { HTMLProps } from 'react';
import type {
  AvailableLocationIpResponse,
  Device,
  DeviceLocationIpsResponse,
  GroupInfo,
  LicenseInfo,
  LicenseTierValue,
  LocationDevicesResponse,
  NetworkDevice,
  NetworkLocation,
  OpenIdClient,
  StartEnrollmentResponse,
  User,
  Webhook,
} from '../../api/types';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';

export interface OpenConfirmActionModal {
  title: string;
  contentMd: string;
  actionPromise: () => Promise<unknown>;
  invalidateKeys?: QueryKey[];
  cancelProps?: ButtonProps;
  submitProps?: ButtonProps;
  contentContainerProps?: HTMLProps<HTMLDivElement>;
  onSuccess?: () => void;
  onError?: () => void;
}

export interface OpenEditDeviceModal {
  device: Device;
  reservedNames: string[];
  reservedPubkeys: string[];
  username: string;
}

export interface OpenAuthKeyRenameModal {
  id: number;
  name: string;
  username: string;
  reservedNames: string[];
}

export interface OpenAddApiTokenModal {
  username: string;
}

export interface OpenRenameApiTokenModal {
  id: number;
  name: string;
  username: string;
}

export interface OpenDeleteApiTokenModal {
  id: number;
  username: string;
}

export interface OpenCEGroupModal {
  groupInfo?: GroupInfo;
  reservedNames: string[];
  users: User[];
}

export interface OpenEditUserModal {
  user: User;
  reservedUsernames: string[];
  reservedEmails: string[];
}

export interface OpenCEOpenIdClientModal {
  openIdClient?: OpenIdClient;
  reservedNames: string[];
}

export interface OpenEnrollmentTokenModal {
  user: User;
  appInfo: {
    smtp_enabled: boolean;
  };
  enrollmentResponse: StartEnrollmentResponse;
}

export interface OpenAddNewDeviceModal {
  user: User;
}

export interface OpenCEWebhookModal {
  webhook?: Webhook;
}

export interface OpenAssignUsersToGroupsModal {
  users: number[];
  groups: GroupInfo[];
}

export interface OpenAddNetworkDeviceModal {
  locations: NetworkLocation[];
  availableIps: AvailableLocationIpResponse;
  reservedNames: string[];
}
export interface OpenEditNetworkDeviceModal {
  device: NetworkDevice;
  reservedNames: string[];
}
export interface OpenNetworkDeviceConfigModal {
  device: NetworkDevice;
  config: string;
}
export interface OpenNetworkDeviceTokenModal {
  device: NetworkDevice;
  enrollment: StartEnrollmentResponse;
}

export interface OpenDisplayListModal {
  title?: string;
  data: string[];
}

export interface OpenDeleteAliasDestinationConfirmModal {
  title: string;
  description: string;
  target: {
    kind: 'alias' | 'destination';
    id: number;
  };
}

export interface OpenDeleteAliasDestinationBlockedModal {
  title: string;
  description: string;
  rules: string[];
}

export interface OpenSettingsLicenseModal {
  license?: string | null;
}

export interface OpenLicenseLimitConflictModal {
  conflicts: Array<{
    label: string;
    current: number;
    limit: number;
  }>;
}

export interface OpenLicenseExpiredModal {
  licenseTier: LicenseTierValue;
}

export interface OpenAddLocationModal {
  license: LicenseInfo | null;
}

export interface OpenAssignUserIPModal {
  user: User;
  locationData: LocationDevicesResponse;
  hasDevices: boolean;
}

export interface OpenAssignUserDeviceIPModal {
  device: Device;
  username: string;
  locationData: DeviceLocationIpsResponse;
}

export interface OpenDeleteLocationModal {
  id: number;
  name: string;
}

export interface OpenDeleteNetworkDeviceModal {
  id: number;
  name: string;
}

export interface OpenDeleteOpenIdClientModal {
  client_id: string;
  name: string;
}
