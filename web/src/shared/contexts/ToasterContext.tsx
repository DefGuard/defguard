import { createContext, ReactNode } from 'react';
import ReactDOM from 'react-dom';
import { Subject } from 'rxjs';

import {
  CustomToastContentProps,
} from '../components/layout/Toast/Toast';

export interface ToasterContextValue {
  eventObserver: Subject<CustomToastContentProps>;
}

const contextDefaultValue: ToasterContextValue = {
  eventObserver: new Subject(),
};

export const ToasterContext =
  createContext<ToasterContextValue>(contextDefaultValue);

interface Props {
  children?: ReactNode;
}



export const ToastsManager = () => {
  const element = document.getElementById("toasts-root");
  if(!element) return null;
  return ReactDOM.createPortal(<></>, element);
};
