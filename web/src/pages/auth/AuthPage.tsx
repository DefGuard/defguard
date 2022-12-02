import './style.scss';

import { useEffect } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';
import shallow from 'zustand/shallow';

import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import Login from './Login/Login';
import { MFARoute } from './MFARoute/MFARoute';

const AuthPage = () => {
  const navigate = useNavigate();

  const [loggedUser, isAdmin, authLocation] = useAuthStore(
    (state) => [state.user, state.isAdmin, state.authLocation],
    shallow
  );

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
        <SvgDefguardLogoLogin />
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
