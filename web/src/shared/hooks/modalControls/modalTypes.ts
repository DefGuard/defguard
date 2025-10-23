import z from 'zod';
import type { User } from '../../api/types';

export const ModalName = {
  ChangePassword: 'changePassword',
  TotpSetup: 'totpSetup',
  RecoveryCodes: 'recoveryCodes',
  EmailMfaSetup: 'emailMfaSetup',
  WebauthnSetup: 'webauthnSetup',
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
]);

export type ModalOpenEvent = z.infer<typeof modalOpenArgsSchema>;
