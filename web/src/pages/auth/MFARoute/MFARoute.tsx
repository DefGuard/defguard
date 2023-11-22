import './style.scss';

import { useEffect } from 'react';
import { Route, Routes, useNavigate } from 'react-router';

import { useI18nContext } from '../../../i18n/i18n-react';
import { UserMFAMethod } from '../../../shared/types';
import { useMFAStore } from '../shared/hooks/useMFAStore';
import { MFAEmail } from './MFAEmail/MFAEmail';
import { MFANav } from './MFANav/MFANav';
import { MFARecovery } from './MFARecovery/MFARecovery';
import { MFATOTPAuth } from './MFATOTPAuth/MFATOTPAuth';
import { MFAWeb3 } from './MFAWeb3/MFAWeb3';
import { MFAWebAuthN } from './MFAWebAuthN/MFAWebAuthN';

export const MFARoute = () => {
  const { LL } = useI18nContext();
  return (
    <section id="mfa">
      <h1>{LL.loginPage.mfa.title()}</h1>
      <Routes>
        <Route index element={<RedirectToDefaultMFA />} />
        <Route path="totp" element={<MFATOTPAuth />} />
        <Route path="webauthn" element={<MFAWebAuthN />} />
        <Route path="web3" element={<MFAWeb3 />} />
        <Route path="email" element={<MFAEmail />} />
        <Route path="recovery" element={<MFARecovery />} />
        <Route path="/*" element={<RedirectToDefaultMFA />} />
      </Routes>
      <MFANav />
    </section>
  );
};

const RedirectToDefaultMFA = () => {
  const defaultMFAMethod = useMFAStore((state) => state.mfa_method);
  const navigate = useNavigate();

  useEffect(() => {
    switch (defaultMFAMethod) {
      case UserMFAMethod.WEB3:
        navigate('/auth/mfa/web3', { replace: true });
        break;
      case UserMFAMethod.WEB_AUTH_N:
        navigate('/auth/mfa/webauthn', { replace: true });
        break;
      case UserMFAMethod.ONE_TIME_PASSWORD:
        navigate('/auth/mfa/totp', { replace: true });
        break;
      case UserMFAMethod.EMAIL:
        navigate('/auth/mfa/email', { replace: true });
        break;
      default:
        navigate('/auth/login', { replace: true });
        break;
    }
  }, [defaultMFAMethod, navigate]);

  return <></>;
};
