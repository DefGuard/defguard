import { useCallback, useMemo, useRef } from 'react';
import { LoginPage } from '../../shared/components/LoginPage/LoginPage';
import './style.scss';
import { useLoaderData, useSearch } from '@tanstack/react-router';
import { m } from '../../paraglide/messages';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { parseOpenIdScopeSearch } from './utils';

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

  const scope = useMemo(() => parseOpenIdScopeSearch(search.scope), [search]);

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
        {scope.map((scope) => {
          const translation_fn = m[`openid_consent_scope_${scope}`];
          // If backend returns scope that was not defined by us we won't have translation for this, TSC only guarantees translation for defined scopes in types
          const display = translation_fn?.() ?? scope;
          return (
            <li key={scope}>
              <Icon size={18} icon="check-circle" />
              <p>{display}</p>
            </li>
          );
        })}
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
