import './style.scss';

import { motion } from 'framer-motion';
import { cloneDeep } from 'lodash-es';
import { useEffect, useRef, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import Badge, { BadgeStyleVariant } from '../../shared/components/layout/Badge/Badge';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import SvgIconCheckmarkWhite from '../../shared/components/svg/IconCheckmarkWhite';
import SvgIconDelete from '../../shared/components/svg/IconDelete';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useToaster } from '../../shared/hooks/useToaster';
import { standardVariants } from '../../shared/variants';

export const OAuthPage = () => {
  const toaster = useToaster();
  const [params] = useSearchParams();
  const [scope, setScope] = useState<string | null>('');
  const [responseType, setResponseType] = useState<string | null>('');
  const [clientId, setClientId] = useState<string | null>('');
  const [codeChallenge, setCodeChallenge] = useState<string | null>('');
  const [codeChallengeMethod, setCodeChallengeMethod] = useState<string | null>('');
  const [redirectUri, setRedirectUri] = useState<string | null>('');
  const [state, setState] = useState<string | null>('');
  const inputRef = useRef<HTMLInputElement | null>(null);
  const currentUser = useAuthStore((state) => state.user);
  const setAuthStore = useAuthStore((state) => state.setState);
  const navigate = useNavigate();
  const authLocation = useAuthStore((state) => state.openIdState);

  useEffect(() => {
    if (!currentUser) {
      const loc = String(cloneDeep(window.location.href));
      setAuthStore({ openIdState: loc });
      setTimeout(() => {
        navigate('/auth', { replace: true });
      }, 250);
    } else {
      if (authLocation) {
        setAuthStore({ openIdState: undefined });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    setScope(params.get('scope'));
    setResponseType(params.get('response_type'));
    setClientId(params.get('client_id'));
    setCodeChallenge(params.get('code_challenge'));
    setCodeChallengeMethod(params.get('code_challenge_method'));
    setState(params.get('state'));
    setRedirectUri(params.get('redirect_uri'));
  }, [params]);

  const getFormAction = (allow: boolean) => {
    if (validateParams()) {
      const res = params;
      res.append('allow', String(allow));
      return `/api/v1/oauth/authorize?${res.toString()}`;
    } else {
      toaster.error('Invalid options.');
    }
    return '';
  };

  const handleSubmit = (allow: boolean) => {
    const formAction = getFormAction(allow);
    if (inputRef.current) {
      inputRef.current.formAction = formAction;
      inputRef.current.click();
    }
  };

  const validateParams = (): boolean => {
    const check = [
      scope,
      responseType,
      clientId,
      codeChallenge,
      codeChallengeMethod,
      redirectUri,
      state,
    ];

    for (const item in check) {
      if (typeof item === 'undefined' || typeof item === null) {
        return false;
      }
    }
    return true;
  };

  return (
    <motion.section
      initial="hidden"
      animate="show"
      variants={standardVariants}
      id="oauth-consent"
    >
      <div className="defguard-logo">
        <SvgDefguardLogoLogin />
      </div>
      <div className="content">
        <h1>Confirm permissions</h1>
        <p>
          Grant permissions for client:
          <span className="client-id">{clientId}</span>
        </p>
        <div className="scopes">
          <p>In scopes :</p>
          {scope && scope.length
            ? scope
                .split(' ')
                .map((s) => (
                  <Badge key={s} text={s} styleVariant={BadgeStyleVariant.PRIMARY} />
                ))
            : null}
        </div>
        <div className="controls">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
            icon={<SvgIconCheckmarkWhite />}
            text="Accept"
            onClick={() => handleSubmit(true)}
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.WARNING}
            text="Decline"
            icon={<SvgIconDelete />}
            onClick={() => handleSubmit(false)}
          />
        </div>
      </div>
      <form method="post">
        <input type="submit" ref={inputRef} />
      </form>
    </motion.section>
  );
};
