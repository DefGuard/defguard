import './style.scss';

import { Route, Routes } from 'react-router';

import { MFACode } from './MFACode/MFACode';
import { MFAKey } from './MFAKey/MFAKey';
import { MFAWallet } from './MFAWallet/MFAWallet';

export const MFARoute = () => {
  return (
    <section id="mfa">
      <h1>Two-factor authentication</h1>
      <Routes>
        <Route path="code" element={<MFACode />} />
        <Route path="key" element={<MFAKey />} />
        <Route path="wallet" element={<MFAWallet />} />
      </Routes>
    </section>
  );
};
