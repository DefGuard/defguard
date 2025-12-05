import { useCallback, useRef } from 'react';
import { LoginPage } from '../../shared/components/LoginPage/LoginPage';
import './style.scss';
import { useLoaderData, useSearch } from '@tanstack/react-router';
import { m } from '../../paraglide/messages';
import type { OpenIdClientScopeValue } from '../../shared/api/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';

export const OpenIdConsentPage = () => {
  const search = useSearch({ from: '/consent' });
  const { data: openIdClient } = useLoaderData({ from: '/consent' });
  const inputRef = useRef<HTMLInputElement>(null);

  const handleClick = useCallback((allow: boolean) => {
    if (isPresent(inputRef.current)) {
      const base = '/api/v1/oauth/authorize?';
      const searchParams = new URLSearchParams(window.location.search);
      searchParams.append('allow', String(allow));
      const formAction = base + searchParams.toString();
      inputRef.current.formAction = formAction;
      inputRef.current.click();
    }
  }, []);

  return (
    <LoginPage id="openid-consent-page">
      <h1>
        {m.openid_consent_title({
          name: openIdClient.name,
        })}
        :
      </h1>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul>
        {(search.scope.trim().split(',') as Array<OpenIdClientScopeValue>).map(
          (scope) => (
            <li key={scope}>
              <Icon size={18} icon="check-circle" />
              <p>{m[`openid_consent_scope_${scope}`]()}</p>
            </li>
          ),
        )}
      </ul>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        text={m.controls_accept()}
        testId="accept-openid"
        onClick={() => handleClick(true)}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Button
        variant="critical"
        text={m.controls_dont_allow()}
        onClick={() => handleClick(false)}
      />
      <form method="post">
        <input type="submit" ref={inputRef} />
      </form>
    </LoginPage>
  );
};
