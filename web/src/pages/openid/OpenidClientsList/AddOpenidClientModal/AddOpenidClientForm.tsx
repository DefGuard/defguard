import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
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
import useApi from '../../../../shared/hooks/useApi';
//import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';

interface Inputs {
  name: string;
  redirect_uri: string[];
  enabled: string | number;
  scope: string[];
}

interface Props {
  setIsOpen: (v: boolean) => void;
}

const AddOpenidClientForm: React.FC<Props> = ({ setIsOpen }) => {
  const { t } = useTranslation('en');
  const {
    openid: { addOpenidClient },
  } = useApi();

  const [scopes, setScopes] = useState<string[]>([]);

  const schema = yup
    .object({
      name: yup
        .string()
        .required(t('form.errors.required'))
        .max(16, t('form.errors.maximumLength', { length: 16 })),
      enabled: yup.boolean(),
    })
    .required();

  const { handleSubmit, control, register } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      name: '',
      redirect_uri: [''],
      scope: ['openid'],
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

  const handleScopeChange = (scope: string, value: boolean) => {
    if (value === true) {
      setScopes([...scopes, scope]);
    } else {
      if (scope.includes(scope)) {
        setScopes(scopes.filter((item) => item !== scope));
      }
    }
  };
  const { fields, append, prepend, remove, swap, move, insert } = useFieldArray(
    {
      control, // control props comes from useForm (optional: if you are using FormContext)
      name: 'redirect_uri', // unique name for your Field Array
    }
  );

  const onSubmit: SubmitHandler<Inputs> = (data) => {
		data.redirect_uri = [data.redirect_uri];
    addOpenidClientMutation.mutate(data);
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        outerLabel="Name"
        placeholder="Name"
        required
      />
      <FormInput
        outerLabel="Redirect Url"
        controller={{ control, name: 'redirect_uri' }}
        placeholder="https://example.com/redirect"
        required
      />
      <label>Scopes:</label>
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
