import { Subject } from 'rxjs';
import type { User } from '../../api/types';

const ModalName = {
  ChangePasswordAdmin: 'changePasswordAdmin',
  ChangePasswordUser: 'changePasswordUser',
} as const;

export type ModalNameValue = (typeof ModalName)[keyof typeof ModalName];

export type ModalOpenArgs = {
  [ModalName.ChangePasswordAdmin]: { user: User };
  [ModalName.ChangePasswordUser]: { user: User };
};

type ModalOpenEvent<N extends keyof ModalOpenArgs = keyof ModalOpenArgs> = {
  name: N;
  data: ModalOpenArgs[N];
};

export const openModalSubject = new Subject<ModalOpenEvent>();

export const closeModalSubject = new Subject<ModalNameValue>();
