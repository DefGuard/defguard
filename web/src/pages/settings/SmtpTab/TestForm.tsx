import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { patternValidEmail } from '../../../shared/patterns';
import { SMTPError, TestMail } from '../../../shared/types';

export const TestForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    mail: { sendTestMail },
  } = useApi();

  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isLoading } = useMutation([], sendTestMail, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.smtp.testForm.controls.success());
    },
    onError: (err: SMTPError) => {
      toaster.error(
        `${LL.settingsPage.smtp.testForm.controls.error()}`,
        `${err.response?.data.error}`
      );
      console.error(err);
    },
  });
  const testFormSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          to: yup.string().matches(patternValidEmail, LL.form.error.invalid()),
        })
        .required(),
    [LL.form.error]
  );

  const { control: testControl, handleSubmit: handleTestSubmit } = useForm<TestMail>({
    defaultValues: {
      to: '',
    },
    resolver: yupResolver(testFormSchema),
    mode: 'all',
  });

  const onSubmit: SubmitHandler<TestMail> = async (data) => {
    mutate(data);
  };

  return (
    <>
      <header>
        <h3>{LL.settingsPage.smtp.testForm.title()}</h3>
      </header>
      <form id="smtp-test-form" onSubmit={handleTestSubmit(onSubmit)}>
        <FormInput
          outerLabel={LL.settingsPage.smtp.testForm.fields.to.label()}
          controller={{ control: testControl, name: 'to' }}
          placeholder={LL.settingsPage.smtp.testForm.fields.to.placeholder()}
          required
        />
        <div className="controls">
          <Button
            text={
              breakpoint !== 'mobile'
                ? LL.settingsPage.smtp.testForm.controls.submit()
                : undefined
            }
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            type="submit"
          />
        </div>
      </form>
    </>
  );
};
