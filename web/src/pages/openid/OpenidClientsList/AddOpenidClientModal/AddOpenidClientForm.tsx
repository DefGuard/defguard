import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useFieldArray, useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import * as yup from 'yup';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { CheckBox } from '../../../../shared/components/layout/Checkbox/CheckBox';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';

interface Inputs {
  name: string;
  redirect_uri: { url: string }[];
  enabled: string | number;
  scope: string[];
}

const AddOpenidClientForm = () => {
  const { t } = useTranslation('en');
  const {
    openid: { addOpenidClient },
  } = useApi();

  const [scopes, setScopes] = useState<string[]>(['']);
  const setModalState = useModalStore((state) => state.setAddOpenidClientModal);

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
          url: yup.string().required(t('form.errors.required')),
        })
        .required()
    ),
  });

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      name: '',
      redirect_uri: [{ url: '' }],
      scope: [],
      enabled: 1,
    },
  });
  const queryClient = useQueryClient();
  const addOpenidClientMutation = useMutation(addOpenidClient, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      setModalState({ visible: false });
    },
  });

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

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    const redirectUrls = data.redirect_uri.map((obj) => {
      return obj['url'];
    });
    const payload = {
      name: data.name,
      redirect_uri: redirectUrls,
      scope: scopes,
      enabled: true,
    };

    addOpenidClientMutation.mutate(payload);
  };

  return (
    <form id="openid-client" onSubmit={handleSubmit(onSubmit)}>
      <div className="row">
        <div className="item">
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
                  key={field.id}
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
        </div>
      </div>
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
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={() => setModalState({ visible: false })}
          type="button"
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Add app"
        />
      </div>
    </form>
  );
};

export default AddOpenidClientForm;
