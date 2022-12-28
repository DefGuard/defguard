/* eslint-disable @typescript-eslint/ban-ts-comment */
import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

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
  const toaster = useToaster();
  const {
    webhook: { addWebhook, editWebhook },
  } = useApi();
  const modalState = useModalStore((state) => state.webhookModal);
  const setModalState = useModalStore((state) => state.setWebhookModal);
  const editMode = useMemo(
    () => !isUndefined(modalState.webhook),
    [modalState.webhook]
  );
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
            .required('Url is required.')
            .matches(patternValidUrl, 'Enter a valid url'),
          description: yup
            .string()
            .min(4, 'Should be at least 4 characters long.')
            .max(65, 'Maximum length exceeded.')
            .required(),
          token: yup
            .string()
            .required('Token is required.')
            .matches(patternAtLeastOneDigit, 'Should have at least one digit.')
            .matches(
              patternAtLeastOneUpperCaseChar,
              'Should have at least one upper case character.'
            )
            .matches(
              patternAtLeastOneSpecialChar,
              'Should have at least one special character.'
            )
            .matches(
              patternAtLeastOneLowerCaseChar,
              'Should have at least one small character.'
            )
            .max(250, 'Maximum length exceeded.'),
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
    []
  );

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues: defaultFormState,
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  const { mutate: addWebhookMutation, isLoading: addWebhookIsLoading } =
    useMutation([MutationKeys.EDIT_WEBHOOK], addWebhook, {
      onSuccess: () => {
        toaster.success('Webhook added.');
        setModalState({ visible: false, webhook: undefined });
      },
      onError: (err) => {
        toaster.error('Error has occurred.');
        setModalState({ visible: false, webhook: undefined });
        console.error(err);
      },
    });
  const { mutate: editWebhookMutation, isLoading: editMutationIsLoading } =
    useMutation([MutationKeys.EDIT_WEBHOOK], editWebhook, {
      onSuccess: () => {
        toaster.success('Webhook modified.');
        setModalState({ visible: false, webhook: undefined });
      },
      onError: (err) => {
        toaster.error('Error has occurred.');
        setModalState({ visible: false, webhook: undefined });
        console.error(err);
      },
    });

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
        outerLabel="Url"
        controller={{ control, name: 'url' }}
        placeholder="https://example.com/webhook_trigger"
        required
      />
      <FormInput
        outerLabel="Description"
        controller={{ control, name: 'description' }}
        placeholder="Description"
        required
        type="text"
      />
      <FormInput
        outerLabel="Secret token"
        controller={{ control, name: 'token' }}
        placeholder="Authorization token"
        required
      />
      <h3>Trigger events:</h3>
      <div className="events">
        <FormCheckBox
          controller={{ control, name: 'on_user_created' }}
          label="New user created"
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_deleted' }}
          label="User deleted"
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_modified' }}
          label="User modified"
          labelPosition="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_hwkey_provision' }}
          label="User Yubikey provision"
          labelPosition="right"
        />
      </div>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
          type="button"
          text="Cancel"
          className="cancel"
          onClick={() => setModalState({ visible: false, webhook: undefined })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.BIG}
          type="submit"
          className="submit"
          text={editMode ? 'Edit webhook' : 'Add webhook'}
          loading={addWebhookIsLoading || editMutationIsLoading}
        />
      </div>
    </form>
  );
};
