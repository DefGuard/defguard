import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useModalStore } from '../../../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../../shared/hooks/useToaster';
import { patternNumbersOnly } from '../../../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../../../shared/queries';
import { useEmailMFAModal } from '../../hooks/useEmailMFAModal';

type FormFields = {
  code: string;
};

const defaultValues: FormFields = {
  code: '',
};

export const RegisterMFAEmailForm = () => {
  const { LL } = useI18nContext();
  const [disableResend, setDisableResend] = useState(false);
  const setModalsState = useModalStore((state) => state.setState);
  const closeModal = useEmailMFAModal((state) => state.close);

  const {
    auth: {
      mfa: {
        email: {
          register: { start, finish },
        },
      },
    },
  } = useApi();

  const toaster = useToaster();
  const queryClient = useQueryClient();

  const localLL = LL.modals.registerEmailMFA;

  const schema = useMemo(
    () =>
      z.object({
        code: z
          .string()
          .regex(patternNumbersOnly, LL.form.error.invalid())
          .min(6, LL.form.error.minimumLength())
          .max(6, LL.form.error.maximumLength()),
      }),
    [LL.form.error],
  );

  const { handleSubmit, control, setError } = useForm<FormFields>({
    defaultValues,
    resolver: zodResolver(schema),
  });

  const { mutate: mutateFinish, isLoading: finishLoading } = useMutation({
    mutationFn: finish,
    onSuccess: (res) => {
      toaster.success(localLL.messages.success());
      queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
      if (res && res.codes) {
        setModalsState({
          recoveryCodesModal: { visible: true, codes: res.codes },
        });
      }
      closeModal();
    },
    onError: () => {
      setError(
        'code',
        {
          type: 'validate',
          message: localLL.form.fields.code.error(),
        },
        {
          shouldFocus: true,
        },
      );
    },
  });

  const { isLoading: initStartLoading } = useQuery({
    queryFn: start,
    queryKey: ['FETCH_INIT_MFA_EMAIL_REGISTER'],
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const { mutateAsync: mutateStart, isLoading: startLodaing } = useMutation({
    mutationFn: start,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutateFinish({
      code: Number.parseInt(data.code),
    });
  };

  return (
    <>
      <form id="register-mfa-email-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          type="text"
          inputMode="numeric"
          controller={{ control, name: 'code' }}
          label={localLL.form.fields.code.label()}
        />
        <div className="form-extras">
          <Button
            className="resend"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.LINK}
            loading={startLodaing || initStartLoading}
            disabled={disableResend}
            text={localLL.form.controls.resend()}
            onClick={() => {
              mutateStart().then(() => {
                toaster.success(localLL.messages.resend());
              });
              setDisableResend(true);
              setTimeout(() => {
                if (setDisableResend) {
                  setDisableResend(false);
                }
              }, 5000);
            }}
          />
        </div>
        <div className="controls">
          <Button
            className="cancel"
            size={ButtonSize.LARGE}
            text={LL.common.controls.cancel()}
            onClick={() => closeModal()}
          />
          <Button
            type="submit"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            className="submit"
            text={localLL.form.controls.submit()}
            loading={finishLoading || startLodaing || initStartLoading}
          />
        </div>
      </form>
    </>
  );
};
