/* eslint-disable @typescript-eslint/no-non-null-assertion */
import './style.scss';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import SvgIconCheckmarkWhite from '../../shared/components/svg/IconCheckmarkWhite';
import SvgIconDelete from '../../shared/components/svg/IconDelete';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';

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
  const currentUser = useAuthStore((state) => state.user);
  const setAuthStore = useAuthStore((state) => state.setState);
  const navigate = useNavigate();
  const authLocation = useAuthStore((state) => state.authLocation);

  const validateParams = useCallback(() => {
    const check = [scope, responseType, clientId, nonce, redirectUri, state];
    for (const item in check) {
      if (typeof item === 'undefined' || typeof item === null) {
        return false;
      }
    }
    return true;
  }, [scope, responseType, clientId, nonce, redirectUri, state]);

  useEffect(() => {
    if (!currentUser) {
      const loc = window.location.href;
      setAuthStore({ authLocation: loc });
      setTimeout(() => {
        navigate('/auth', { replace: true });
      }, 250);
    } else {
      if (authLocation) {
        setAuthStore({ authLocation: undefined });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentUser]);
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
    openid: 'Use your profile data for future logins.',
    profile:
      'Know basic information from your profile like name, profile picture etc.',
    email: 'Know your email address.',
    phone: 'Know your phone number.',
  };

  return (
    <section id="openid-consent">
      <div className="logo-container">
        <SvgDefguardLogoLogin />
      </div>
      <div className="consent">
        <h1>{name} would like to:</h1>
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
            text="Accept"
            onClick={() => handleSubmit(true)}
          />
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.STANDARD}
            icon={<SvgIconDelete />}
            text="Cancel"
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
