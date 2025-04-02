import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmark from '../../../../../../shared/components/svg/IconCheckmark';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import useApi from '../../../../../../shared/hooks/useApi';
import { patternValidEmail } from '../../../../../../shared/patterns';
import { TestMail } from '../../../../../../shared/types';
import { useSmtpTestModal } from './useSmtpTestModal';

type SMTPError = AxiosError<{ error: string }>;

export const SmtpTestModal = () => {
  const { LL } = useI18nContext();
  const modal = useSmtpTestModal((s) => s);
  const {
    mail: { sendTestMail },
  } = useApi();
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const {
    mutate,
    isPending: isLoading,
    status,
    reset: resetMutation,
  } = useMutation({
    mutationFn: sendTestMail,
    onError: (err: SMTPError) => {
      if (err.response?.data.error) {
        setErrorMessage(err.response?.data.error);
      }
      console.error(err);
    },
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        to: z.string().regex(patternValidEmail, LL.form.error.invalid()),
      }),
    [LL.form.error],
  );

  const { control: testControl, handleSubmit: handleTestSubmit } = useForm<TestMail>({
    defaultValues: {
      to: '',
    },
    resolver: zodResolver(zodSchema),
    mode: 'all',
  });

  const onSubmit: SubmitHandler<TestMail> = (data) => {
    mutate(data);
  };

  return (
    <ModalWithTitle
      isOpen={modal.visible}
      onClose={() => {
        modal.close();
      }}
      afterClose={() => {
        modal.reset();
        resetMutation();
        setErrorMessage(null);
      }}
      backdrop
      disableClose
    >
      <div id="smtp-test-mail">
        <header>
          <h2>{LL.settingsPage.smtp.testForm.title()}</h2>
        </header>
        {status === 'success' && (
          <p className="success">{LL.settingsPage.smtp.testForm.success.message()}</p>
        )}
        {status === 'error' && (
          <div>
            <p className="error">{LL.settingsPage.smtp.testForm.error.message()}</p>
            <p>
              {LL.settingsPage.smtp.testForm.error.fullError({
                error: errorMessage || 'Unknown error',
              })}
            </p>
          </div>
        )}
        {(status === 'idle' || status === 'pending') && (
          <>
            <p>{LL.settingsPage.smtp.testForm.subtitle()}</p>
            <form id="smtp-test-form" onSubmit={handleTestSubmit(onSubmit)}>
              <FormInput
                label={LL.settingsPage.smtp.testForm.fields.to.label()}
                controller={{ control: testControl, name: 'to' }}
                placeholder={LL.settingsPage.smtp.testForm.fields.to.placeholder()}
                required
              />
            </form>
          </>
        )}
        <div className="button-row">
          <Button
            text={
              status === 'idle' ? LL.common.controls.cancel() : LL.common.controls.close()
            }
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => {
              modal.close();
            }}
          />
          <Button
            text={
              status === 'idle'
                ? LL.settingsPage.smtp.testForm.controls.submit()
                : LL.settingsPage.smtp.testForm.controls.retry()
            }
            icon={status === 'idle' ? <IconCheckmark /> : undefined}
            styleVariant={ButtonStyleVariant.PRIMARY}
            loading={isLoading}
            type="submit"
            form="smtp-test-form"
            onClick={() => {
              if (status === 'error') {
                resetMutation();
              }
            }}
          />
        </div>
      </div>
    </ModalWithTitle>
  );
};
