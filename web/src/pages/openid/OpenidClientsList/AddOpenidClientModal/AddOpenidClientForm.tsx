import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import React from 'react';
import { useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import * as yup from 'yup';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import useApi from '../../../../shared/hooks/useApi';
import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';

interface Inputs {
  name: string;
  description: string;
  home_url: string;
  redirect_uri: string;
  enabled: string | number;
}

interface Props {
  setIsOpen: (v: boolean) => void;
}

const AddOpenidClientForm: React.FC<Props> = ({ setIsOpen }) => {
  const { t } = useTranslation('en');
  const {
    openid: { addOpenidClient },
  } = useApi();

  const schema = yup
    .object({
      name: yup
        .string()
        .required(t('form.errors.required'))
        .max(16, t('form.errors.maximumLength', { length: 16 })),
      home_url: yup
        .string()
        .required(t('form.errors.required'))
        .matches(patternValidUrl, t('form.errors.invalidUrl')),
      description: yup
        .string()
        .required(t('form.errors.required'))
        .max(30, t('form.errors.minimumLength', { length: 30 })),
      redirect_uri: yup
        .string()
        .required(t('form.errors.required'))
        .matches(patternValidUrl, t('form.errors.invalidUrl')),
      enabled: yup.boolean(),
    })
    .required();

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      name: '',
      home_url: '',
      description: '',
      redirect_uri: '',
      enabled: 1,
    },
  });
  const queryClient = useQueryClient();
  const addOpenidClientMutation = useMutation(addOpenidClient, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);

      setIsOpen(false);
    },
  });

  const onSubmit: SubmitHandler<Inputs> = (data) =>
    addOpenidClientMutation.mutate(data);

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        outerLabel="Name"
        placeholder="Name"
        required
      />
      <FormInput
        outerLabel="Home Url"
        controller={{ control, name: 'home_url' }}
        placeholder="https://example.com"
        required
      />
      <FormInput
        outerLabel="Description"
        controller={{ control, name: 'description' }}
        placeholder="Description"
        required
      />
      <FormInput
        outerLabel="Redirect Url"
        controller={{ control, name: 'redirect_uri' }}
        placeholder="https://example.com/redirect"
        required
      />
      <Button
        className="big primary"
        type="submit"
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text="Add app"
      />
    </form>
  );
};

export default AddOpenidClientForm;
