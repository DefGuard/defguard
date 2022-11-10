/* eslint-disable @typescript-eslint/no-non-null-assertion */
import './style.scss';

import { AxiosResponse } from 'axios';
import { motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useLocation, useSearchParams } from 'react-router-dom';
import { Navigate } from 'react-router-dom';
import { toast } from 'react-toastify';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import SvgDefguadNavLogo from '../../shared/components/svg/DefguadNavLogo';
import SvgIconCheckmarkWhite from '../../shared/components/svg/IconCheckmarkWhite';
import SvgIconDelete from '../../shared/components/svg/IconDelete';
import ToastContent, {
  ToastType,
} from '../../shared/components/layout/Toast/Toast';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { patternBaseUrl } from '../../shared/patterns';
import { standardVariants } from '../../shared/variants';
import LoaderPage from '../loader/LoaderPage';

const OpenidAllowPage: React.FC = () => {
  const [params] = useSearchParams();
  const [scope, setScope] = useState<string | null>('');
  const [responseType, setResponseType] = useState<string | null>('');
  const [clientId, setClientId] = useState<string | null>('');
  const [nonce, setNonce] = useState<string | null>('');
  const [redirectUri, setRedirectUri] = useState<string | null>('');
  const [state, setState] = useState<string | null>('');
  const [isLoading, setLoading] = useState<boolean>(true);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const {
    openid: { verifyOpenidClient },
  } = useApi();
  const location = useLocation();
  const path = location.pathname + location.search;
  const currentUser = useAuthStore((state) => state.user);

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
        return `/api/v1/openid/authorize?${res.toString()}`;
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
      const verifyOpenidClientRequest = {
        scope: params.get('scope')!,
        response_type: params.get('response_type')!,
        client_id: params.get('client_id')!,
        nonce: params.get('nonce')!,
        redirect_uri: params.get('redirect_uri')!,
        state: params.get('state')!,
        allow: false,
      };
      verifyOpenidClient(verifyOpenidClientRequest)
        .then((res: AxiosResponse) => {
          if (res.status == 200) {
            handleSubmit(true);
          }
        })
        .catch(() => setLoading(false));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [validateParams, handleSubmit, params]);

  if (isUndefined(currentUser)) {
    return <Navigate replace to="/auth/login" state={{ path: path }} />;
  }

  const scopes: Record<string, string> = {
    profile:
      'Know basic information from your profile like first name and last name.',
    email: 'Know your email adres.',
    phone: 'Know your phone number.',
  };

  const domain = params.get('redirect_uri')?.match(patternBaseUrl)![1];

  return (
    <>
      {isLoading ? <LoaderPage /> : null}
      <motion.section
        initial="hidden"
        animate="show"
        variants={standardVariants}
        id="openid-consent"
      >
        <div className="content">
          <div className="header">
            <SvgDefguadNavLogo />
          </div>
          <h1>{domain} would like to:</h1>
          <div className="scopes-container">
            {scope && scope.length
              ? scope
                  .split(' ')
                  .filter((scope) => scope != 'openid' && scope.length > 3)
                  .map((s) => (
                    <div className="scope" key={s}>
                      <p className="text">{scopes[s]}</p>
                    </div>
                  ))
              : null}
          </div>
          <div className="footer">
            <p className="disclaimer">
              By clicking accept button you&apos;re allowing {domain} to read
              above information from your Defguard account.
            </p>
            <div className="controls">
              <Button
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.STANDARD}
                icon={<SvgIconDelete />}
                text="Cancel"
                onClick={() => handleSubmit(false)}
              />
              <Button
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.PRIMARY}
                icon={<SvgIconCheckmarkWhite />}
                text="Accept"
                onClick={() => handleSubmit(true)}
              />
            </div>
          </div>
        </div>
        <form method="post">
          <input type="submit" ref={inputRef} />
        </form>
      </motion.section>
    </>
  );
};

export default OpenidAllowPage;
