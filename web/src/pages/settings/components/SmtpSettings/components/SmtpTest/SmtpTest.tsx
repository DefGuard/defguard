import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmark from '../../../../../../shared/components/svg/IconCheckmark';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { patternValidEmail } from '../../../../../../shared/patterns';
import { TestMail } from '../../../../../../shared/types';

type SMTPError = AxiosError<{ error: string }>;

export const SmtpTest = () => {
  const submitRef = useRef<HTMLInputElement | null>(null);
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    mail: { sendTestMail },
  } = useApi();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: sendTestMail,
    onSuccess: () => {
      toaster.success(LL.settingsPage.smtp.testForm.controls.success());
    },
    onError: (err: SMTPError) => {
      toaster.error(
        `${LL.settingsPage.smtp.testForm.controls.error()}`,
        `${err.response?.data.error}`,
      );
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
    <section id="smtp-test-mail">
      <header>
        <h2>{LL.settingsPage.smtp.testForm.title()}</h2>
        <Button
          text={LL.settingsPage.smtp.testForm.controls.submit()}
          icon={<IconCheckmark />}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          loading={isLoading}
          type="submit"
          onClick={() => {
            if (!isLoading && submitRef.current) {
              submitRef?.current?.click();
            }
          }}
        />
      </header>
      <form id="smtp-test-form" onSubmit={void handleTestSubmit(onSubmit)}>
        <FormInput
          label={LL.settingsPage.smtp.testForm.fields.to.label()}
          controller={{ control: testControl, name: 'to' }}
          placeholder={LL.settingsPage.smtp.testForm.fields.to.placeholder()}
          required
        />
        <input type="submit" className="hidden" ref={submitRef} />
      </form>
    </section>
  );
};
