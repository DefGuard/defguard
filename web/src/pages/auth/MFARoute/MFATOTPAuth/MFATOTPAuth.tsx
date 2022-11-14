import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

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

const schema = yup
  .object()
  .shape({
    code: yup
      .string()
      .required('Code is required.')
      .min(6, 'Code should have 6 digits')
      .max(6, 'Code should have 6 digits'),
  })
  .required();

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

  const { mutate, isLoading } = useMutation(
    [MutationKeys.VERIFY_TOTP],
    verify,
    {
      onSuccess: (data) => {
        clearMFAStore();
        logIn(data);
      },
      onError: (err) => {
        console.error(err);
        setValue('code', '');
        setError('code', { message: 'Enter a valid code' });
      },
    }
  );

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
      <p>Use code from your authentication app and click button to proceed</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'code' }}
          autoComplete="one-time-code"
          placeholder="Enter Authenticator code"
          required
        />
        <Button
          text="Use authenticator code"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading}
          type="submit"
        />
      </form>
      {web3Available || webauthnAvailable ? (
        <nav>
          <span>or</span>
          {webauthnAvailable && (
            <Button
              text="Use security key instead"
              size={ButtonSize.BIG}
              styleVariant={ButtonStyleVariant.LINK}
              onClick={() => navigate('../webauthn')}
            />
          )}
          {web3Available && (
            <Button
              text="Use your wallet instead"
              size={ButtonSize.BIG}
              styleVariant={ButtonStyleVariant.LINK}
              onClick={() => navigate('../web3')}
            />
          )}
        </nav>
      ) : null}
    </>
  );
};
