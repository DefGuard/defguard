import '../src/shared/scss/ladleStyles.scss';
import './fonts.css';

import type { GlobalProvider } from '@ladle/react';

export const Provider: GlobalProvider = ({ children }) => {
  return <>{children}</>;
};
