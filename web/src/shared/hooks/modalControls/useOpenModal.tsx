import { useCallback } from 'react';
import {
  type ModalNameValue,
  type ModalOpenArgs,
  openModalSubject,
} from './modalsSubjects';

export const useOpenModal = <K extends ModalNameValue>(name: K) => {
  type Arg = ModalOpenArgs[K];

  const open = useCallback(
    (data: Arg) => {
      openModalSubject.next({
        name,
        data,
      });
    },
    [name],
  );

  return open;
};
