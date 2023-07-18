/* eslint-disable @typescript-eslint/no-non-null-assertion */
import './style.scss';

import { AxiosError } from 'axios';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { useI18nContext } from '../../i18n/i18n-react';
import { Button } from '../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/types';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import SvgIconCheckmarkWhite from '../../shared/components/svg/IconCheckmarkWhite';
import SvgIconDelete from '../../shared/components/svg/IconDelete';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { LoaderPage } from '../loader/LoaderPage';

export const OpenidAllowPage = () => {
  const navigate = useNavigate();

  const [allowLoading, setAllowLoading] = useState(false);
  const [cancelLoading, setCancelLoading] = useState(false);
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
  const setAuthStore = useAuthStore((state) => state.setState);
  const [loadingInfo, setLoadingInfo] = useState(true);
  const toaster = useToaster();

  const { LL } = useI18nContext();

  const paramsValid = useMemo(() => {
    const check = [scope, responseType, clientId, nonce, redirectUri, state];
    for (const item in check) {
      if (typeof item === 'undefined' || typeof item === null) {
        toaster.error('OpenID Params invalid.');
        return false;
      }
    }
    return true;
  }, [clientId, nonce, redirectUri, responseType, scope, state, toaster]);

  const handleSubmit = useCallback(
    (allow: boolean) => {
      params.append('allow', String(allow));
      const formAction = `/api/v1/oauth/authorize?${params.toString()}`;
      if (inputRef.current) {
        inputRef.current.formAction = formAction;
        inputRef.current.click();
      }
    },
    [params],
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
    if (paramsValid && clientId) {
      getOpenidClient(clientId)
        .then((res) => {
          setName(res.name);
          setLoadingInfo(false);
        })
        .catch((error: AxiosError) => {
          if (error.response?.status === 401) {
            setAuthStore({ openIdParams: params });
            setLoadingInfo(false);
            navigate('/auth', { replace: true });
          } else {
            navigate('/', { replace: true });
          }
        });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paramsValid, clientId]);

  const scopes: Record<string, string> = {
    openid: LL.openidAllow.scopes.openid(),
    profile: LL.openidAllow.scopes.profile(),
    email: LL.openidAllow.scopes.email(),
    phone: LL.openidAllow.scopes.phone(),
  };

  if (loadingInfo) return <LoaderPage />;

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
            data-testid="openid-allow"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconCheckmarkWhite />}
            text={LL.openidAllow.controls.accept()}
            disabled={!paramsValid}
            loading={allowLoading}
            onClick={() => {
              setAllowLoading(true);
              handleSubmit(true);
            }}
          />
          <Button
            data-testid="openid-cancel"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.STANDARD}
            icon={<SvgIconDelete />}
            text={LL.openidAllow.controls.cancel()}
            disabled={!paramsValid}
            loading={cancelLoading}
            onClick={() => {
              setCancelLoading(true);
              handleSubmit(false);
            }}
          />
        </div>
      </div>
      <form method="post">
        <input type="submit" ref={inputRef} />
      </form>
    </section>
  );
};
