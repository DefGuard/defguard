import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/types';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { RecoveryLoginRequest } from '../../../../shared/types';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFARecovery = () => {
  const toaster = useToaster();
  const navigate = useNavigate();
  const [totpAvailable, web3Available, webauthnAvailable] = useMFAStore(
    (state) => [state.totp_available, state.web3_available, state.webauthn_available],
    shallow
  );
  const loginSubject = useAuthStore((state) => state.loginSubject);
  const {
    auth: {
      mfa: { recovery },
    },
  } = useApi();

  const { LL } = useI18nContext();

  const { mutate, isLoading } = useMutation([MutationKeys.RECOVERY_LOGIN], recovery, {
    onSuccess: (data) => loginSubject.next(data),
    onError: (err) => {
      console.error(err);
      toaster.error('Recovery code invalid.');
    },
  });

  const schema = yup
    .object()
    .shape({
      code: yup.string().required(LL.form.error.required()),
    })
    .required();

  const { handleSubmit, control } = useForm<RecoveryLoginRequest>({
    resolver: yupResolver(schema),
    defaultValues: {
      code: '',
    },
    mode: 'all',
  });

  useEffect(() => {
    if (!totpAvailable && !web3Available && !webauthnAvailable) {
      navigate('../');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleValidSubmit: SubmitHandler<RecoveryLoginRequest> = (values) =>
    mutate(values);

  return (
    <>
      <p>{LL.loginPage.mfa.recoveryCode.header()}</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          placeholder={LL.loginPage.mfa.recoveryCode.form.fields.code.placeholder()}
          controller={{ control, name: 'code' }}
        />
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.loginPage.mfa.recoveryCode.form.controls.submit()}
          loading={isLoading}
        />
      </form>
      <nav>
        <span>or</span>
        {totpAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useAuthenticator()}
            size={ButtonSize.LARGE}
            onClick={() => navigate('../totp')}
          />
        )}
        {webauthnAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useWebauthn()}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => navigate('../webauthn')}
          />
        )}
        {web3Available && (
          <Button
            text={LL.loginPage.mfa.controls.useWallet()}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => navigate('../web3')}
          />
        )}
      </nav>
    </>
  );
};
