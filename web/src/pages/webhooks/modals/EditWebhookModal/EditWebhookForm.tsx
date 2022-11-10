import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { toast } from 'react-toastify';
import * as yup from 'yup';

import { FormCheckBox } from '../../../../shared/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../shared/components/layout/MessageBox/MessageBox';
import ToastContent, {
  ToastType,
} from '../../../../shared/components/layout/Toast/Toast';
import useApi from '../../../../shared/hooks/useApi';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
  patternValidUrl,
} from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';
import { Webhook } from '../../../../shared/types';

interface Inputs {
  id: string;
  url: string;
  description: string;
  token: string;
  enabled: string | number;
  on_user_created: string | number;
  on_user_deleted: string | number;
  on_user_modified: string | number;
  on_hwkey_provision: string | number;
}

interface Props {
  setIsOpen: (v: boolean) => void;
  webhook: Webhook;
}

const AddWebhookForm: React.FC<Props> = ({ setIsOpen, webhook }) => {
  const { t } = useTranslation('en');
  const {
    webhook: { editWebhook },
  } = useApi();
  const formSchema = yup.object({
    url: yup
      .string()
      .required(t('form.errors.required'))
      .matches(patternValidUrl, t('form.errors.invalidUrl')),
    description: yup
      .string()
      .min(4, t('form.errors.minimumLength', { length: 4 }))
      .max(30, t('form.errors.maximumLength', { length: 30 }))
      .required(),
    token: yup
      .string()
      .required(t('form.errors.required'))
      .matches(patternAtLeastOneDigit, t('form.errors.atLeastOneDigit'))
      .matches(
        patternAtLeastOneUpperCaseChar,
        t('form.errors.atLeastOneUpperCaseChar')
      )
      .matches(
        patternAtLeastOneSpecialChar,
        t('form.errors.atLeastOneSpecialChar')
      )
      .matches(
        patternAtLeastOneLowerCaseChar,
        t('form.errors.atLeastOneLowerCaseChar')
      )
      .max(30, t('form.errors.maximumLength', { length: 40 })),
    enabled: yup.boolean(),
    on_user_created: yup.boolean(),
    on_user_deleted: yup.boolean(),
    on_user_modified: yup.boolean(),
    on_password_sent: yup.boolean(),
    on_hwkey_provision: yup.boolean(),
  });

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    defaultValues: {
      id: webhook.id,
      url: webhook.url,
      description: webhook.description,
      token: webhook.token,
      enabled: webhook.enabled ? 1 : 0,
      on_user_created: webhook.on_user_created ? 1 : 0,
      on_user_deleted: webhook.on_user_deleted ? 1 : 0,
      on_user_modified: webhook.on_user_modified ? 1 : 0,
      on_hwkey_provision: webhook.on_hwkey_provision ? 1 : 0,
    },
  });
  const queryClient = useQueryClient();
  const editWebhookMutation = useMutation(editWebhook, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_WEBHOOKS]); 
      setIsOpen(false);
    },
    onError: () => {
      setIsOpen(false);
    },
  });

  const [error, setError] = useState('');

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    if (
      data.on_hwkey_provision ||
      data.on_user_modified ||
      data.on_user_deleted ||
      data.on_user_created
    ) {
      editWebhookMutation.mutate(data);
    } else {
      setError('Select at least one trigger');
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
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
      />
      <FormInput
        outerLabel="Secret token"
        controller={{ control, name: 'token' }}
        placeholder="Authorization token"
        required
      />
      <div className="triggers-container">
        <label>Triggers:</label>
        <FormCheckBox
          controller={{ control, name: 'on_user_created' }}
          label="New user created"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_deleted' }}
          label="User deleted"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_modified' }}
          label="User modified"
        />
        <FormCheckBox
          controller={{ control, name: 'on_hwkey_provision' }}
          label="User Yubikey provision"
        />
        <div className="errors-container">
          {error ? (
            <MessageBox message={error} type={MessageBoxType.ERROR} />
          ) : null}
        </div>
      </div>
      <Button
        className="big primary"
        type="submit"
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text="Edit webhook"
      />
    </form>
  );
};

export default AddWebhookForm;
