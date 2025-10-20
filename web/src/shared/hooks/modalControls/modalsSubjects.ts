import { filter, Subject } from 'rxjs';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import type { ModalNameValue, ModalOpenArgs } from './types';

type ModalOpenEvent<N extends keyof ModalOpenArgs = keyof ModalOpenArgs> = {
  name: N;
  data: ModalOpenArgs[N];
};

export const openModalSubject = new Subject<ModalOpenEvent>();

export const closeModalSubject = new Subject<ModalNameValue>();

export const subscribeOpenModal = <N extends ModalNameValue>(
  name: N,
  handler: (data: ModalOpenArgs[N]) => void,
) => {
  return openModalSubject
    .pipe(filter((e) => isPresent(e.name) && e.name === name))
    .subscribe((e) => handler(e.data));
};

export const subscribeCloseModal = <N extends ModalNameValue>(
  name: N,
  handler: () => void,
) => {
  return closeModalSubject.pipe(filter((e) => e === name)).subscribe(() => handler());
};

// convenient aliases

export const closeModal = <T extends ModalNameValue>(name: T) => {
  closeModalSubject.next(name);
};

export const openModal = <N extends ModalNameValue>(name: N, data: ModalOpenArgs[N]) => {
  openModalSubject.next({
    name,
    data,
  });
};
