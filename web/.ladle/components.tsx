import type { GlobalProvider } from '@ladle/react';
import '../src/shared/scss/ladleStyles.scss';

export const Provider: GlobalProvider = ({ children }) => <>{children}</>;
