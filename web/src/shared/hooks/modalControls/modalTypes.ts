import z from 'zod';
import type { ActivityLogStream, AddDeviceResponse, User } from '../../api/types';
import type {
  OpenAddApiTokenModal,
  OpenAddLocationModal,
  OpenAddNetworkDeviceModal,
  OpenAssignUserDeviceIPModal,
  OpenAssignUserIPModal,
  OpenAssignUsersToGroupsModal,
  OpenAuthKeyRenameModal,
  OpenCEGroupModal,
  OpenCEOpenIdClientModal,
  OpenCEWebhookModal,
  OpenDisplayListModal,
  OpenEditDeviceModal,
  OpenEditNetworkDeviceModal,
  OpenEditUserModal,
  OpenEnrollmentTokenModal,
  OpenLicenseExpiredModal,
  OpenNetworkDeviceConfigModal,
  OpenNetworkDeviceTokenModal,
  OpenRenameApiTokenModal,
  OpenSettingsLicenseModal,
} from './types';

export const ModalName = {
  LicenseExpired: 'licenseExpired',
  UpgradeBusiness: 'upgradeBusiness',
  UpgradeEnterprise: 'upgradeEnterprise',
  LimitReached: 'limitReached',
  SettingsLicense: 'settingsLicense',
  SendTestMail: 'sendTestMail',
  GatewaySetup: 'gatewaySetup',
  DisplayList: 'displayList',
  ChangePassword: 'changePassword',
  TotpSetup: 'totpSetup',
  RecoveryCodes: 'recoveryCodes',
  EmailMfaSetup: 'emailMfaSetup',
  WebauthnSetup: 'webauthnSetup',
  EditUserDevice: 'editUserDevice',
  UserDeviceConfig: 'userDeviceConfig',
  AddAuthKey: 'addAuthKey',
  RenameAuthKey: 'renameAuthKey',
  AddApiToken: 'addApiToken',
  RenameApiToken: 'renameApiToken',
  CreateEditGroup: 'createEditGroup',
  EditUserModal: 'editUserModal',
  CEOpenIdClient: 'createEditOpenIdClient',
  CEWebhook: 'createEditWebhook',
  AssignGroupsToUsers: 'assignGroupsToUsers',
  AddNetworkDevice: 'addNetworkDevice',
  EditNetworkDevice: 'editNetworkDevice',
  NetworkDeviceConfig: 'networkDeviceConfig',
  NetworkDeviceToken: 'networkDeviceToken',
  AddLocation: 'addLocation',
  AddLogStreaming: 'addLogStreaming',
  EditLogStreaming: 'editLogStreaming',
  DeleteLogStreaming: 'deleteLogStreaming',
  SelfEnrollmentToken: 'selfEnrollmentToken',
  AddNewDevice: 'addNewDevice',
  AssignUserIP: 'assignUserIP',
  AssignUserDeviceIP: 'assignUserDeviceIP',
} as const;

export type ModalNameValue = (typeof ModalName)[keyof typeof ModalName];

const modalOpenArgsSchema = z.discriminatedUnion('name', [
  z.object({
    name: z.literal(ModalName.ChangePassword),
    data: z.object({
      user: z.custom<User>(),
      adminForm: z.boolean(),
    }),
  }),
  z.object({ name: z.literal(ModalName.TotpSetup) }),
  z.object({ name: z.literal(ModalName.RecoveryCodes), data: z.array(z.string()) }),
  z.object({ name: z.literal(ModalName.EmailMfaSetup) }),
  z.object({ name: z.literal(ModalName.WebauthnSetup) }),
  z.object({
    name: z.literal(ModalName.EditUserDevice),
    data: z.custom<OpenEditDeviceModal>(),
  }),
  z.object({
    name: z.literal(ModalName.UserDeviceConfig),
    data: z.custom<AddDeviceResponse>(),
  }),
  z.object({
    name: z.literal(ModalName.AddAuthKey),
    data: z.object({
      username: z.string(),
    }),
  }),
  z.object({
    name: z.literal(ModalName.RenameAuthKey),
    data: z.custom<OpenAuthKeyRenameModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddApiToken),
    data: z.custom<OpenAddApiTokenModal>(),
  }),
  z.object({
    name: z.literal(ModalName.RenameApiToken),
    data: z.custom<OpenRenameApiTokenModal>(),
  }),
  z.object({
    name: z.literal(ModalName.CreateEditGroup),
    data: z.custom<OpenCEGroupModal>(),
  }),
  z.object({
    name: z.literal(ModalName.EditUserModal),
    data: z.custom<OpenEditUserModal>(),
  }),
  z.object({
    name: z.literal(ModalName.SelfEnrollmentToken),
    data: z.custom<OpenEnrollmentTokenModal>(),
  }),
  z.object({
    name: z.literal(ModalName.EditUserModal),
    data: z.custom<OpenEditUserModal>(),
  }),
  z.object({
    name: z.literal(ModalName.CEOpenIdClient),
    data: z.custom<OpenCEOpenIdClientModal>(),
  }),
  z.object({
    name: z.literal(ModalName.CEWebhook),
    data: z.custom<OpenCEWebhookModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AssignGroupsToUsers),
    data: z.custom<OpenAssignUsersToGroupsModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddNetworkDevice),
    data: z.custom<OpenAddNetworkDeviceModal>(),
  }),
  z.object({
    name: z.literal(ModalName.EditNetworkDevice),
    data: z.custom<OpenEditNetworkDeviceModal>(),
  }),
  z.object({
    name: z.literal(ModalName.NetworkDeviceConfig),
    data: z.custom<OpenNetworkDeviceConfigModal>(),
  }),
  z.object({
    name: z.literal(ModalName.NetworkDeviceToken),
    data: z.custom<OpenNetworkDeviceTokenModal>(),
  }),
  z.object({
    name: z.literal(ModalName.DisplayList),
    data: z.custom<OpenDisplayListModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddLocation),
    data: z.custom<OpenAddLocationModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddLogStreaming),
  }),
  z.object({
    name: z.literal(ModalName.EditLogStreaming),
    data: z.custom<ActivityLogStream>(),
  }),
  z.object({
    name: z.literal(ModalName.DeleteLogStreaming),
    data: z.custom<ActivityLogStream>(),
  }),
  z.object({
    name: z.literal(ModalName.SendTestMail),
  }),
  z.object({
    name: z.literal(ModalName.SettingsLicense),
    data: z.custom<OpenSettingsLicenseModal>(),
  }),
  z.object({
    name: z.literal(ModalName.LimitReached),
  }),
  z.object({
    name: z.literal(ModalName.UpgradeBusiness),
  }),
  z.object({
    name: z.literal(ModalName.UpgradeEnterprise),
  }),
  z.object({
    name: z.literal(ModalName.LicenseExpired),
    data: z.custom<OpenLicenseExpiredModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddNewDevice),
    data: z.custom<User>(),
  }),
  z.object({
    name: z.literal(ModalName.AssignUserIP),
    data: z.custom<OpenAssignUserIPModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AssignUserDeviceIP),
    data: z.custom<OpenAssignUserDeviceIPModal>(),
  }),
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
