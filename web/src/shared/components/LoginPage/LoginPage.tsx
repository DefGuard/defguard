import './style.scss';
import dayjs from 'dayjs';
import type { PropsWithChildren } from 'react';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import asci_image from './assets/login_asci.png';
import { LoginPageLogo } from './LoginPageLogo';

export const LoginPage = ({ children }: PropsWithChildren) => {
  return (
    <div id="login-page">
      <aside>
        <img src={asci_image} />
      </aside>
      <div className="main-track">
        <main>
          <LoginPageLogo />
          <SizedBox height={ThemeSpacing.Xl8} />
          {children}
          <footer>
            <p>
              Copyright © {dayjs().year()}{' '}
              <a href="https://defguard.net" target="_blank" rel="noopener">
                Defguard
              </a>{' '}
              Sp. z o.o.
            </p>
          </footer>
        </main>
      </div>
    </div>
  );
};
