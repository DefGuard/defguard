import { filter, Subject, type Subscription } from 'rxjs';
import type { ModalNameValue, ModalOpenEvent } from './modalTypes';

export const openModalSubject = new Subject<ModalOpenEvent>();

export const closeModalSubject = new Subject<ModalNameValue>();

export function subscribeOpenModal<N extends ModalOpenEvent['name']>(
  name: N,
  handler: Extract<ModalOpenEvent, { name: N }> extends { data: infer D }
    ? (data: D) => void
    : () => void,
): Subscription {
  return openModalSubject
    .pipe(filter((e): e is Extract<ModalOpenEvent, { name: N }> => e.name === name))
    .subscribe((e) => {
      if ('data' in e) (handler as (d: unknown) => void)(e.data);
      else (handler as () => void)();
    });
}

export const subscribeCloseModal = <N extends ModalNameValue>(
  name: N,
  handler: () => void,
) => {
  return closeModalSubject.pipe(filter((e) => e === name)).subscribe(() => handler());
};

export const closeModal = <T extends ModalNameValue>(name: T) => {
  closeModalSubject.next(name);
};

type PayloadOf<N extends ModalNameValue> =
  Extract<ModalOpenEvent, { name: N }> extends {
    data: infer D;
  }
    ? D
    : undefined;
export function openModal<N extends ModalNameValue>(
  name: N,
  ...args: PayloadOf<N> extends undefined ? [] : [PayloadOf<N>]
) {
  const event = (
    args.length === 0 ? { name } : { name, data: args[0] }
  ) as ModalOpenEvent;

  // blur whatever right now has focus
  const activeElement = document.activeElement as HTMLElement | null;
  activeElement?.blur();

  openModalSubject.next(event);
}
