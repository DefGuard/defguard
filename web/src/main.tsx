import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './shared/defguard-ui/scss/index.scss';
import { App } from './app/App.tsx';

// biome-ignore lint/style/noNonNullAssertion: always there
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
