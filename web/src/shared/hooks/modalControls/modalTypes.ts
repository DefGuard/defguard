import z from 'zod';
import type { AddDeviceResponse, User } from '../../api/types';
import type { OpenAuthKeyRenameModal, OpenEditDeviceModal } from './types';

export const ModalName = {
  ChangePassword: 'changePassword',
  TotpSetup: 'totpSetup',
  RecoveryCodes: 'recoveryCodes',
  EmailMfaSetup: 'emailMfaSetup',
  WebauthnSetup: 'webauthnSetup',
  EditUserDevice: 'editUserDevice',
  UserDeviceConfig: 'userDeviceConfig',
  AddAuthKey: 'addAuthKey',
  RenameAuthKey: 'renameAuthKey',
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
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
