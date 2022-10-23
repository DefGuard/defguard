/* eslint-disable @typescript-eslint/no-explicit-any */
import React from 'react';

import IconButton from '../layout/IconButton/IconButton';
import { IconPopupClose } from '../svg';

/**
 * Replaces default close button from react-toastify package
 */
const ToastifyCloseButton = ({ closeToast }: any): React.ReactElement => (
  <IconButton className="popup-close blank" onClick={closeToast}>
    <IconPopupClose />
  </IconButton>
);

export default ToastifyCloseButton;
