import type { User } from '../../api/types';

const ModalName = {
  ChangePassword: 'changePassword',
} as const;

export type ModalNameValue = (typeof ModalName)[keyof typeof ModalName];

export type ModalOpenArgs = {
  [ModalName.ChangePassword]: { user: User; adminForm: boolean };
};
