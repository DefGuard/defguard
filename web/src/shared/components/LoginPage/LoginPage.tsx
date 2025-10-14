import './style.scss';
import type { PropsWithChildren } from 'react';
import asci_image from './assets/login_asci.png';

export const LoginPage = ({ children }: PropsWithChildren) => {
  return (
    <div id="login-page">
      <aside>
        <img src={asci_image} />
      </aside>
      <div className="main-track">
        <main>{children}</main>
      </div>
    </div>
  );
};
