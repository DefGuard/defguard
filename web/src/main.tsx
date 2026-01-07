import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './shared/defguard-ui/scss/index.scss';
import 'react-loading-skeleton/dist/skeleton.css';
import { App } from './app/App.tsx';

// biome-ignore lint/style/noNonNullAssertion: always there
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
