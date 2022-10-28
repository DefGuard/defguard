import './style.scss';

import { Route, Routes } from 'react-router';

import { MFAKey } from './MFAKey/MFAKey';
import { MFATOTPAuth } from './MFATOTPAuth/MFATOTPAuth';
import { MFAWallet } from './MFAWallet/MFAWallet';
import { MFAWeb3SignMessageModal } from './modals/MFAWeb3SignModal';

export const MFARoute = () => {
  return (
    <section id="mfa">
      <h1>Two-factor authentication</h1>
      <Routes>
        <Route path="code" element={<MFATOTPAuth />} />
        <Route path="key" element={<MFAKey />} />
        <Route path="wallet" element={<MFAWallet />} />
      </Routes>
      <MFAWeb3SignMessageModal />
    </section>
  );
};
