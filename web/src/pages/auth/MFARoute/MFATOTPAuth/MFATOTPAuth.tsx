import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';
import shallow from 'zustand/shallow';
import { useI18nContext } from '../../../../i18n/i18n-react';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

interface Inputs {
  code: string;
}



export const MFATOTPAuth = () => {
  const navigate = useNavigate();
  const clearMFAStore = useMFAStore((state) => state.resetState);
  const [totpAvailable, web3Available, webauthnAvailable] = useMFAStore(
    (state) => [
      state.totp_available,
      state.web3_available,
      state.webauthn_available,
    ],
    shallow
  );
  const logIn = useAuthStore((state) => state.logIn);
  const {
    auth: {
      mfa: {
        totp: { verify },
      },
    },
  } = useApi();
  const { LL } = useI18nContext();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.VERIFY_TOTP],
    verify,
    {
      onSuccess: (data) => {
        const { user, url } = data;
        if (user && url) {
          clearMFAStore();
          logIn(user);
          window.location.replace(url);
          return;
        }
        if (user) {
          clearMFAStore();
          logIn(user);
        }
      },
      onError: (err) => {
        console.error(err);
        setValue('code', '');
        setError('code', { message: 'Enter a valid code' });
      },
    }
  );
const schema = yup
  .object()
  .shape({
    code: yup
      .string()
      .required(LL.form.error.required())
      .min(6, LL.form.error.validCode())
      .max(6, LL.form.error.validCode()),
  })
  .required();

  const { handleSubmit, control, setError, setValue } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      code: '',
    },
  });

  const handleValidSubmit: SubmitHandler<Inputs> = (values) => {
    mutate({ code: Number(values.code) });
  };

  useEffect(() => {
    if (!totpAvailable) {
      navigate('../');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [totpAvailable]);

  return (
    <>
      <p>{LL.loginPage.mfa.totp.header()}</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'code' }}
          autoComplete="one-time-code"
          placeholder={LL.loginPage.mfa.totp.form.fields.code.placeholder()}
          required
        />
        <Button
          text={LL.loginPage.mfa.totp.form.controls.submit()}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading}
          type="submit"
        />
      </form>
      <nav>
        <span>or</span>
        {webauthnAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useWebauthn()}
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => navigate('../webauthn')}
          />
        )}
        {web3Available && (
          <Button
            text={LL.loginPage.mfa.controls.useWallet()}
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => navigate('../web3')}
          />
        )}
        <Button
          text={LL.loginPage.mfa.controls.useRecoveryCode()}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('../recovery')}
        />
      </nav>
    </>
  );
};
