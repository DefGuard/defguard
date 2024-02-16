import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { RecoveryLoginRequest } from '../../../../shared/types';
import { trimObjectStrings } from '../../../../shared/utils/trimObjectStrings';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFARecovery = () => {
  const toaster = useToaster();
  const navigate = useNavigate();
  const [totpAvailable, web3Available, webauthnAvailable, emailAvailable] = useMFAStore(
    (state) => [
      state.totp_available,
      state.web3_available,
      state.webauthn_available,
      state.email_available,
    ],
    shallow,
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

  const zodSchema = useMemo(
    () =>
      z.object({
        code: z.string().min(1, LL.form.error.required()),
      }),
    [LL.form.error],
  );

  const { handleSubmit, control } = useForm<RecoveryLoginRequest>({
    resolver: zodResolver(zodSchema),
    defaultValues: {
      code: '',
    },
    mode: 'all',
  });

  useEffect(() => {
    if (!totpAvailable && !web3Available && !webauthnAvailable && !emailAvailable) {
      navigate('../');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleValidSubmit: SubmitHandler<RecoveryLoginRequest> = (values) =>
    mutate(trimObjectStrings(values));

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
    </>
  );
};
