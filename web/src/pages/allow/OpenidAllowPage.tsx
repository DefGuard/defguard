/* eslint-disable @typescript-eslint/no-non-null-assertion */
import './style.scss';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import SvgIconCheckmarkWhite from '../../shared/components/svg/IconCheckmarkWhite';
import SvgIconDelete from '../../shared/components/svg/IconDelete';
import useApi from '../../shared/hooks/useApi';
import { useI18nContext } from '../../i18n/i18n-react';

export const OpenidAllowPage = () => {
  const [params] = useSearchParams();
  const [scope, setScope] = useState<string | null>('');
  const [responseType, setResponseType] = useState<string | null>('');
  const [clientId, setClientId] = useState<string | null>('');
  const [nonce, setNonce] = useState<string | null>('');
  const [redirectUri, setRedirectUri] = useState<string | null>('');
  const [state, setState] = useState<string | null>('');
  const [name, setName] = useState<string | null>('');
  const inputRef = useRef<HTMLInputElement | null>(null);
  const {
    openid: { getOpenidClient },
  } = useApi();

  const { LL } = useI18nContext();

  const validateParams = useCallback(() => {
    const check = [scope, responseType, clientId, nonce, redirectUri, state];
    for (const item in check) {
      if (typeof item === 'undefined' || typeof item === null) {
        return false;
      }
    }
    return true;
  }, [scope, responseType, clientId, nonce, redirectUri, state]);

  const getFormAction = useCallback(
    (allow: boolean) => {
      if (validateParams()) {
        const res = params;
        res.append('allow', String(allow));
        return `/api/v1/oauth/authorize?${res.toString()}`;
      }
      return '';
    },
    [validateParams, params]
  );

  const handleSubmit = useCallback(
    (allow: boolean) => {
      const formAction = getFormAction(allow);
      if (inputRef.current) {
        inputRef.current.formAction = formAction;
        inputRef.current.click();
      }
    },
    [getFormAction]
  );

  useEffect(() => {
    setScope(params.get('scope'));
    setResponseType(params.get('response_type'));
    setClientId(params.get('client_id'));
    setNonce(params.get('nonce'));
    setState(params.get('state'));
    setRedirectUri(params.get('redirect_uri'));
  }, [params]);

  useEffect(() => {
    if (validateParams()) {
      getOpenidClient(clientId!).then((res) => {
        setName(res.name);
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [validateParams, clientId]);

  const scopes: Record<string, string> = {
    openid: LL.openidAllow.scopes.openid(),
    profile: LL.openidAllow.scopes.profile(),
    email: LL.openidAllow.scopes.email(),
    phone: LL.openidAllow.scopes.phone(),
  };

  return (
    <section id="openid-consent">
      <div className="logo-container">
        <SvgDefguardLogoLogin />
      </div>
      <div className="consent">
        <h1>{LL.openidAllow.header({ name: name || '' })}</h1>
        <ul className="scopes-list">
          {scope && scope.length
            ? scope.split(' ').map((s) => (
                <li className="scope" key={s}>
                  {scopes[s]}
                </li>
              ))
            : null}
        </ul>
        <div className="controls">
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconCheckmarkWhite />}
            text={LL.openidAllow.controls.accept()}
            onClick={() => handleSubmit(true)}
          />
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.STANDARD}
            icon={<SvgIconDelete />}
            text={LL.openidAllow.controls.cancel()}
            onClick={() => handleSubmit(false)}
          />
        </div>
      </div>
      <form method="post">
        <input type="submit" ref={inputRef} />
      </form>
    </section>
  );
};
