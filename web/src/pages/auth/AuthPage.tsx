import './style.scss';

import { useEffect } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';
import shallow from 'zustand/shallow';

import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import Login from './Login/Login';
import { MFARoute } from './MFARoute/MFARoute';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';

const AuthPage = () => {
  const navigate = useNavigate();

  const [loggedUser, isAdmin, authLocation] = useAuthStore(
    (state) => [state.user, state.isAdmin, state.authLocation],
    shallow
  );
  const settings = useAppStore((state) => state.settings);

  useEffect(() => {
    if (loggedUser) {
      if (authLocation) {
        location.assign(authLocation);
      } else {
        if (loggedUser && isAdmin) {
          navigate('/admin/overview', { replace: true });
        } else {
          navigate('/me', { replace: true });
        }
      }
    }
  }, [isAdmin, loggedUser, navigate, authLocation]);

  return (
    <div id="auth-container">
      <div className="logo-container">
        {settings ? (
          <img src={settings?.main_logo_url} />
        ) : (
          <SvgDefguardLogoLogin />
        )}
      </div>
      <Routes>
        <Route index element={<Navigate to="login" />} />
        <Route path="/" element={<Navigate to="login" />} />
        <Route path="login" element={<Login />} />
        <Route path="mfa/*" element={<MFARoute />} />
        <Route path="*" element={<Navigate to="login" />} />
      </Routes>
    </div>
  );
};

export default AuthPage;
