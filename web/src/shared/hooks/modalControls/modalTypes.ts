import z from 'zod';
import type { ActivityLogStream, AddDeviceResponse, User } from '../../api/types';
import type {
  OpenAddApiTokenModal,
  OpenAddNetworkDeviceModal,
  OpenAssignUsersToGroupsModal,
  OpenAuthKeyRenameModal,
  OpenCEGroupModal,
  OpenCEOpenIdClientModal,
  OpenCEWebhookModal,
  OpenDisplayListModal,
  OpenEditDeviceModal,
  OpenEditNetworkDeviceModal,
  OpenEditUserModal,
  OpenGatewaySetupModal,
  OpenLicenseModal,
  OpenNetworkDeviceConfigModal,
  OpenNetworkDeviceTokenModal,
  OpenRenameApiTokenModal,
  OpenUpgradeLicenseModal,
} from './types';

export const ModalName = {
  UpgradeLicenseModal: 'upgradeLicenseModal',
  License: 'license',
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
    name: z.literal(ModalName.GatewaySetup),
    data: z.custom<OpenGatewaySetupModal>(),
  }),
  z.object({
    name: z.literal(ModalName.AddLocation),
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
    name: z.literal(ModalName.License),
    data: z.custom<OpenLicenseModal>(),
  }),
  z.object({
    name: z.literal(ModalName.UpgradeLicenseModal),
    data: z.custom<OpenUpgradeLicenseModal>(),
  }),
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
