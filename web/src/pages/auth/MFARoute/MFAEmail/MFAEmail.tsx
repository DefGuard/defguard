import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import { z } from 'zod';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { patternNumbersOnly } from '../../../../shared/patterns';
import { trimObjectStrings } from '../../../../shared/utils/trimObjectStrings';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

type FormFields = {
  code: string;
};

const queryKey = 'SEND_MFA_CODE_EMAIL_LOGIN';

const defaultValues: FormFields = {
  code: '',
};

export const MFAEmail = () => {
  const { LL } = useI18nContext();
  const localLL = LL.loginPage.mfa.email;
  const queryClient = useQueryClient();
  const [resendEnabled, setResendEnabled] = useState<boolean>(true);
  const navigate = useNavigate();
  const loginSubject = useAuthStore((state) => state.loginSubject);
  const emailAvailable = useMFAStore((state) => state.email_available);

  const {
    auth: {
      mfa: {
        email: { verify, sendCode },
      },
    },
  } = useApi();

  const { isLoading: codeLoading } = useQuery({
    queryFn: sendCode,
    queryKey: [queryKey],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  const schema = useMemo(
    () =>
      z.object({
        code: z
          .string()
          .min(6, LL.form.error.minimumLength())
          .max(6, LL.form.error.maximumLength())
          .regex(patternNumbersOnly, LL.form.error.invalid()),
      }),
    [LL.form.error],
  );

  const { control, handleSubmit, setError, resetField } = useForm<FormFields>({
    defaultValues,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const { mutate: verifyMutate, isLoading: verifyLoading } = useMutation({
    mutationFn: verify,
    onSuccess: (data) => {
      loginSubject.next(data);
    },
    onError: (e) => {
      resetField('code', {
        defaultValue: '',
        keepDirty: true,
        keepError: true,
        keepTouched: true,
      });
      setError('code', {
        message: LL.form.error.invalidCode(),
      });
      console.error(e);
    },
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    const trimmed = trimObjectStrings(data);
    verifyMutate({
      code: String(trimmed.code),
    });
  };

  useEffect(() => {
    if (!emailAvailable) {
      navigate('../');
    }
  }, [emailAvailable, navigate]);

  return (
    <>
      <p>{localLL.header()}</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          type="text"
          inputMode="numeric"
          controller={{ control, name: 'code' }}
          placeholder={localLL.form.labels.code()}
        />
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.submit()}
          disabled={codeLoading}
          loading={verifyLoading}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          text={localLL.form.controls.resendCode()}
          loading={codeLoading}
          disabled={verifyLoading || !resendEnabled}
          onClick={() => {
            queryClient.invalidateQueries([queryKey]);
            setResendEnabled(false);
            setTimeout(() => {
              setResendEnabled(true);
            }, 10000);
          }}
        />
      </form>
    </>
  );
};
