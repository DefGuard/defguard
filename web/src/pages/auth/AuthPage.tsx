import './style.scss';

import React, { useEffect } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';
import shallow from 'zustand/shallow';

import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import Login from './Login/Login';

const AuthPage: React.FC = () => {
  const navigate = useNavigate();

  const [loggedUser, isAdmin] = useAuthStore(
    (state) => [state.user, state.isAdmin],
    shallow
  );

  useEffect(() => {
    if (loggedUser) {
      if (loggedUser && isAdmin) {
        navigate('/admin', { replace: true });
      } else {
        navigate('/me', { replace: true });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section id="auth-container">
      <section className="logo-container">
        <SvgDefguardLogoLogin />
      </section>
      <Routes>
        <Route index element={<Navigate to="login" />} />
        <Route path="/" element={<Navigate to="login" />} />
        <Route path="login" element={<Login />} />
        {/* <Route path="register" element={<Register />} /> */}
        <Route path="*" element={<Navigate to="login" />} />
      </Routes>
    </section>
  );
};

export default AuthPage;
