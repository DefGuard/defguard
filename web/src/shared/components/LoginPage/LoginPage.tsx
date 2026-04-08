import './style.scss';
import dayjs from 'dayjs';
import type { PropsWithChildren } from 'react';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import loginImage from './assets/login.png';
import { LoginPageLogo } from './LoginPageLogo';

export const LoginPage = ({ children, id }: PropsWithChildren & { id?: string }) => {
  return (
    <div id="login-page">
      <aside>
        <img src={loginImage} loading="eager" decoding="async" width={844} height={999} />
      </aside>
      <div className="main-track">
        <main id={id}>
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
