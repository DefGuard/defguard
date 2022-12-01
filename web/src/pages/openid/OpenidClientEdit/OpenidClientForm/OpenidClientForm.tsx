import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { pick as lodashPick } from 'lodash-es';
import React, { useMemo, useRef } from 'react';
import { useEffect } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';
import { Subject } from 'rxjs';
import * as yup from 'yup';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import useApi from '../../../../shared/hooks/useApi';
import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';
import { OpenidClient } from '../../../../shared/types';

interface Inputs {
  name: string;
  description: string;
  redirect_uri: string;
  home_url: string;
  scopes: Scope[];
}

enum Scope {
  profile = 'profile',
  email = 'email',
  phone = 'phone',
}

interface Props {
  client: OpenidClient;
  saveSubject: Subject<unknown>;
  navigateOnSuccess?: boolean;
  onSuccessCallBack?: () => void;
}

const OpenidClientForm: React.FC<Props> = ({
  client,
  saveSubject,
  navigateOnSuccess = true,
  onSuccessCallBack,
}) => {
  const { t } = useTranslation('en');

  const submitRef = useRef<HTMLInputElement>(null);

  const defaultValues = useMemo(
    () =>
      lodashPick(client, ['name', 'home_url', 'description', 'redirect_uri']),
    [client]
  );

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
    })
    .required();

  const {
    openid: { editOpenidClient },
  } = useApi();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const editOpenidClientMutation = useMutation(
    (clientData: OpenidClient) => editOpenidClient(clientData),
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);

        if (navigateOnSuccess) {
          navigate(-1);
        } else {
          if (onSuccessCallBack) {
            onSuccessCallBack();
          }
        }
        reset(defaultValues);
      },
    }
  );
  const { control, handleSubmit, reset } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: defaultValues,
  });

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    editOpenidClientMutation.mutate({ ...client, ...data });
  };

  useEffect(() => {
    if (saveSubject) {
      const sub = saveSubject.subscribe(() => {
        submitRef.current?.click();
      });
      return () => sub.unsubscribe();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [saveSubject]);

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="client-edit-form">
      <div className="labeled-input">
        <label>Name:</label>
        <FormInput
          controller={{ control, name: 'name' }}
          placeholder="Name"
          required
        />
      </div>
      <div className="labeled-input half">
        <label>Description:</label>
        <FormInput
          controller={{ control, name: 'description' }}
          placeholder="Description"
          required
        />
      </div>
      <div className="labeled-input half">
        <label>Home Url:</label>
        <FormInput
          controller={{ control, name: 'home_url' }}
          placeholder="https://example.com"
          required
        />
      </div>
      <div className="labeled-input half">
        <label>Redirect Url:</label>
        <FormInput
          controller={{ control, name: 'redirect_uri' }}
          placeholder="https://example.com/redirect"
          required
        />
      </div>
      <input type="submit" ref={submitRef} />
    </form>
  );
};

export default OpenidClientForm;
