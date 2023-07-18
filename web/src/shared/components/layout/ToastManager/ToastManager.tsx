import './style.scss';

import { useMemo } from 'react';
import ReactDOM from 'react-dom';

import { useToastsStore } from '../../../hooks/store/useToastStore';
import { Toast } from './Toast/Toast';

export const ToastManager = () => {
  const element = useMemo(() => document.getElementById('toasts-root'), []);
  const toasts = useToastsStore((store) => store.toasts);
  if (element === null) return null;
  return ReactDOM.createPortal(
    <>
      {toasts.map((toast) => (
        <Toast key={toast.id} data={toast} />
      ))}
    </>,
    element,
  );
};
