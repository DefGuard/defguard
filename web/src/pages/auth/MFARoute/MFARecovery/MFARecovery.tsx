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
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { RecoveryLoginRequest } from '../../../../shared/types';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

const schema = yup
  .object()
  .shape({
    code: yup.string().required('Field requried'),
  })
  .required();

export const MFARecovery = () => {
  const toaster = useToaster();
  const navigate = useNavigate();
  const clearMFAStore = useMFAStore((state) => state.resetState);
  const logIn = useAuthStore((state) => state.logIn);
  const [totpAvailable, web3Available, webauthnAvailable] = useMFAStore(
    (state) => [
      state.totp_available,
      state.web3_available,
      state.webauthn_available,
    ],
    shallow
  );
  const {
    auth: {
      mfa: { recovery },
    },
  } = useApi();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.RECOVERY_LOGIN],
    recovery,
    {
      onSuccess: (data) => {
        clearMFAStore();
        logIn(data);
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Recovery code invalid.');
      },
    }
  );

  const { handleSubmit, control } = useForm<RecoveryLoginRequest>({
    resolver: yupResolver(schema),
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
      <p>Enter one of active recovery codes and click button to log in.</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          outerLabel="Recovery code"
          placeholder="Code"
          controller={{ control, name: 'code' }}
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Use recovery code"
          loading={isLoading}
        />
      </form>
      <nav>
        <span>or</span>
        {totpAvailable && (
          <Button
            text="Use authenticator app instead"
            size={ButtonSize.BIG}
            onClick={() => navigate('../totp')}
          />
        )}
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
    </>
  );
};
