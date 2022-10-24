import { ToastOptions } from 'react-toastify';

// Configs for react-toastify main container placed in App.tsx

export const standardToastConfig: ToastOptions = {
  position: 'top-right',
  autoClose: 5000,
  hideProgressBar: true,
  icon: false,
};

export const standardToastConfigMobile: ToastOptions = {
  position: 'bottom-center',
  autoClose: 5000,
  hideProgressBar: true,
  icon: false,
};
