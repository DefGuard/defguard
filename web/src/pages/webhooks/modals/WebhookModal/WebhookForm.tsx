/* eslint-disable @typescript-eslint/ban-ts-comment */
import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../shared/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
  patternValidUrl,
} from '../../../../shared/patterns';
import { Webhook } from '../../../../shared/types';

type FormInputs = Omit<Webhook, 'id' | 'enabled'>;

const triggerTest = (value: Partial<FormInputs>): boolean => {
  const keys: (keyof FormInputs)[] = [
    'on_user_modified',
    'on_user_deleted',
    'on_user_created',
    'on_hwkey_provision',
  ];
  let pass = false;
  keys.forEach((key) => {
    if (value[key] === true) {
      pass = true;
    }
  });

  if (!pass) {
    return true;
  }
  return false;
};

const defaultValues: FormInputs = {
  url: '',
  description: '',
  token: '',
  on_hwkey_provision: false,
  on_user_created: false,
  on_user_deleted: false,
  on_user_modified: false,
};

export const WebhookForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    webhook: { addWebhook, editWebhook },
  } = useApi();
  const modalState = useModalStore((state) => state.webhookModal);
  const setModalState = useModalStore((state) => state.setWebhookModal);
  const editMode = useMemo(() => !isUndefined(modalState.webhook), [modalState.webhook]);
  const defaultFormState: FormInputs = useMemo(() => {
    if (!isUndefined(modalState.webhook)) {
      return modalState.webhook;
    }
    return defaultValues;
  }, [modalState.webhook]);

  const formSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          url: yup
            .string()
            .required(LL.modals.webhookModal.form.error.urlRequired)
            .matches(patternValidUrl, LL.modals.webhookModal.form.error.validUrl),
          description: yup
            .string()
            .min(4, LL.form.error.minimumLength)
            .max(65, LL.form.error.maximumLength)
            .required(),
          token: yup
            .string()
            .required(LL.modals.webhookModal.form.error.tokenRequired())
            .matches(patternAtLeastOneDigit, LL.form.error.oneDigit())
            .matches(patternAtLeastOneUpperCaseChar, LL.form.error.oneUppercase())
            .matches(patternAtLeastOneSpecialChar, LL.form.error.oneSpecial())
            .matches(patternAtLeastOneLowerCaseChar, LL.form.error.oneLowercase())
            .max(250, LL.form.error.maximumLength()),
          enabled: yup.boolean(),
          on_user_created: yup.boolean().test({
            message: '',
            //@ts-ignore
            test: (_, parent) => triggerTest(parent as FormInputs),
          }),
          on_user_deleted: yup.boolean().test({
            message: '',
            //@ts-ignore
            test: (_, parent) => triggerTest(parent as FormInputs),
          }),
          on_user_modified: yup.boolean().test({
            message: '',
            //@ts-ignore
            test: (_, parent) => triggerTest(parent as FormInputs),
          }),
          on_hwkey_provision: yup.boolean().test({
            message: '',
            //@ts-ignore
            test: (_, parent) => triggerTest(parent as FormInputs),
          }),
        })
        .required(),
    [LL.form.error, LL.modals.webhookModal.form.error]
  );

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues: defaultFormState,
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  const { mutate: addWebhookMutation, isLoading: addWebhookIsLoading } = useMutation(
    [MutationKeys.EDIT_WEBHOOK],
    addWebhook,
    {
      onSuccess: () => {
        toaster.success(LL.modals.webhookModal.form.messages.successAdd());
        setModalState({ visible: false, webhook: undefined });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false, webhook: undefined });
        console.error(err);
      },
    }
  );
  const { mutate: editWebhookMutation, isLoading: editMutationIsLoading } = useMutation(
    [MutationKeys.EDIT_WEBHOOK],
    editWebhook,
    {
      onSuccess: () => {
        toaster.success(LL.modals.webhookModal.form.messages.successModify());
        setModalState({ visible: false, webhook: undefined });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false, webhook: undefined });
        console.error(err);
      },
    }
  );

  const onValidSubmit: SubmitHandler<FormInputs> = (values) => {
    if (editMode) {
      if (modalState.webhook) {
        editWebhookMutation({ ...modalState.webhook, ...values });
      }
    } else {
      addWebhookMutation({ ...values, enabled: true });
    }
  };

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        outerLabel={LL.modals.webhookModal.form.fields.url.label()}
        controller={{ control, name: 'url' }}
        placeholder={LL.modals.webhookModal.form.fields.url.placeholder()}
        required
      />
      <FormInput
        outerLabel={LL.modals.webhookModal.form.fields.description.label()}
        controller={{ control, name: 'description' }}
        placeholder={LL.modals.webhookModal.form.fields.description.placeholder()}
        required
        type="text"
      />
      <FormInput
        outerLabel={LL.modals.webhookModal.form.fields.token.label()}
        controller={{ control, name: 'token' }}
        placeholder={LL.modals.webhookModal.form.fields.token.placeholder()}
        required
      />
      <h3>{LL.modals.webhookModal.form.triggers()}</h3>
      <div className="events">
        <FormCheckBox
          controller={{ control, name: 'on_user_created' }}
          label={LL.modals.webhookModal.form.fields.userCreated.label()}
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_deleted' }}
          label={LL.modals.webhookModal.form.fields.userDeleted.label()}
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_modified' }}
          label={LL.modals.webhookModal.form.fields.userModified.label()}
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_hwkey_provision' }}
          label={LL.modals.webhookModal.form.fields.hwkeyProvision.label()}
          labelPosition="right"
        />
      </div>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
          type="button"
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalState({ visible: false, webhook: undefined })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.BIG}
          type="submit"
          className="submit"
          text={LL.form.submit()}
          loading={addWebhookIsLoading || editMutationIsLoading}
        />
      </div>
    </form>
  );
};
