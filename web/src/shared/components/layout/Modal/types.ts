import { ReactNode } from 'react';

export interface ModalProps {
  isOpen: boolean;
  // defaults to true
  backdrop?: boolean;
  // depracated use onClose or afterClose
  setIsOpen?: (v: boolean) => void;
  className?: string;
  children?: ReactNode;
  // fires when modal is starting to close
  onClose?: () => void;
  // fires when modal closing animation is done
  afterClose?: () => void;
  id?: string;
  disableClose?: boolean;
  currentStep?: number;
  steps?: ReactNode[];
}
