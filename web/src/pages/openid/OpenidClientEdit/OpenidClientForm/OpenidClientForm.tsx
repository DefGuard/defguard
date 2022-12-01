import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useEffect } from 'react';
import { SubmitHandler, useFieldArray, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';
import { Subject } from 'rxjs';
import * as yup from 'yup';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { CheckBox } from '../../../../shared/components/layout/Checkbox/CheckBox';
import useApi from '../../../../shared/hooks/useApi';
import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';
import { OpenidClient } from '../../../../shared/types';

interface Inputs {
  name: string;
  redirect_uri: [{ url: string }];
  scopes: string[];
}

interface Props {
  client: OpenidClient;
  saveSubject: Subject<unknown>;
  navigateOnSuccess?: boolean;
  onSuccessCallBack?: () => void;
}

const OpenidClientForm = ({
  client,
  saveSubject,
  navigateOnSuccess = true,
  onSuccessCallBack,
}: Props) => {
  const { t } = useTranslation('en');

  const [scopes, setScopes] = useState<string[]>(client.scope ?? []);

  const submitButton = useRef<HTMLButtonElement | null>(null);

  const defaultValues = {
    name: client.name as string | undefined,
    redirect_uri: client.redirect_uri.map((url) => {
      return { url: url };
    }),
    scope: client.scope as string[] | undefined,
  };

  const schema = yup.object({
    name: yup
      .string()
      .required(t('form.errors.required'))
      .max(16, t('form.errors.maximumLength', { length: 16 })),
    enabled: yup.boolean(),
    redirect_uri: yup.array().of(
      yup
        .object()
        .shape({
          url: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternValidUrl, t('form.errors.invalidUrl')),
        })
        .required()
    ),
  });

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
    const redirectUrls = data.redirect_uri.map((el) => {
      return el['url'];
    });
    const payload = {
      name: data.name,
      redirect_uri: redirectUrls,
      scope: scopes,
      enabled: true,
    };

    editOpenidClientMutation.mutate({ ...client, ...payload });
  };

  useEffect(() => {
    if (submitButton && submitButton.current) {
      const sub = saveSubject.subscribe(() => {
        submitButton.current?.click();
      });
      return () => sub.unsubscribe();
    }
  }, [saveSubject]);

  const handleScopeChange = (scope: string, value: boolean) => {
    if (value === true) {
      setScopes([...scopes, scope]);
    } else {
      if (scope.includes(scope)) {
        setScopes(scopes.filter((item) => item !== scope));
      }
    }
  };
  const { fields, append, remove } = useFieldArray({
    control,
    name: 'redirect_uri',
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="client-edit-form">
      <FormInput
        controller={{ control, name: 'name' }}
        outerLabel="Name"
        placeholder="Name"
        required
      />
      {fields.map((field, index) => (
        <>
          <FormInput
            key={field.id}
            outerLabel={`Redirect Url ${index + 1}`}
            controller={{ control, name: `redirect_uri.${index}.url` }}
            placeholder="https://example.com/redirect"
            required
          />
          {index !== 0 ? (
            <Button
              className="big warning"
              type="submit"
              size={ButtonSize.BIG}
              styleVariant={ButtonStyleVariant.WARNING}
              text="Remove redirect uri"
              onClick={() => remove(index)}
            />
          ) : null}
        </>
      ))}
      <Button
        className="big primary"
        type="submit"
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text="Add redirect Url"
        onClick={() => append({ url: '' })}
      />
      <label>Scopes:</label>
      <div className="scopes">
        <CheckBox
          label="OpenID"
          value={scopes.includes('openid')}
          onChange={(value) => handleScopeChange('openid', value)}
        />
        <CheckBox
          label="Profile"
          value={scopes.includes('profile')}
          onChange={(value) => handleScopeChange('profile', value)}
        />
        <CheckBox
          label="Email"
          value={scopes.includes('email')}
          onChange={(value) => handleScopeChange('email', value)}
        />
        <CheckBox
          label="Phone"
          value={scopes.includes('phone')}
          onChange={(value) => handleScopeChange('phone', value)}
        />
      </div>

      <button type="submit" className="hidden" ref={submitButton} />
    </form>
  );
};

export default OpenidClientForm;
