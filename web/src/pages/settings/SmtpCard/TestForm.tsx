import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useI18nContext } from "../../../i18n/i18n-react";
import useApi from "../../../shared/hooks/useApi";
import { useToaster } from "../../../shared/hooks/useToaster";
import { useBreakpoint } from "use-breakpoint";
import { useMemo } from "react";
import * as yup from 'yup';
import { deviceBreakpoints } from "../../../shared/constants";
import { patternValidEmail } from "../../../shared/patterns";
import { SubmitHandler, useForm } from "react-hook-form";
import { TestMail } from "../../../shared/types";
import { yupResolver } from "@hookform/resolvers/yup";
import { FormInput } from "../../../shared/components/Form/FormInput/FormInput";
import { Button } from "../../../shared/components/layout/Button/Button";
import { IconCheckmarkWhite } from "../../../shared/components/svg";
import { ButtonSize, ButtonStyleVariant } from "../../../shared/components/layout/Button/types";

export const TestForm = () => {

  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    mail: { sendTestMail },
  } = useApi();

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isLoading } = useMutation([], sendTestMail, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.smtp.test_form.controls.success());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
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
    <form id="smtp-test-form" onSubmit={handleTestSubmit(onSubmit)}>
      <FormInput
        outerLabel={LL.settingsPage.smtp.test_form.fields.to.label()}
        controller={{ control: testControl, name: 'to' }}
        placeholder={LL.settingsPage.smtp.test_form.fields.to.placeholder()}
        required
      />
      <div className="controls">
        <Button
          text={
            breakpoint !== 'mobile'
              ? LL.settingsPage.smtp.test_form.controls.submit()
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
  )
}
