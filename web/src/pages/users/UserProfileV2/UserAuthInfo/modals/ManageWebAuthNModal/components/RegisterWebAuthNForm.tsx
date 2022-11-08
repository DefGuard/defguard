import {
  create,
  parseCreationOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../../shared/queries';
import { toaster } from '../../../../../../../shared/utils/toaster';

interface FormInputs {
  name: string;
}

const formSchema = yup
  .object()
  .shape({
    name: yup
      .string()
      .required('Name is required')
      .min(4, 'Minimum 4 characters required.'),
  })
  .required();

export const RegisterWebAuthNForm = () => {
  const setModalState = useModalStore((state) => state.setState);
  const [waitingForSecurityKey, setWaitingForSecurityKey] = useState(false);
  const {
    auth: {
      mfa: {
        webauthn: {
          register: { start, finish },
        },
      },
    },
  } = useApi();
  const queryClient = useQueryClient();

  const { mutate: registerKeyFinish, isLoading: registerKeyFinishLoading } =
    useMutation([MutationKeys.REGISTER_SECURITY_KEY_FINISH], finish, {
      onSuccess: () => {
        toaster.success('Security key added.');
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        reset();
      },
      onError: () => {
        toaster.error('Key registration failed.');
        setModalState({ manageWebAuthNKeysModal: { visible: false } });
      },
    });

  const { mutate: registerKeyStart, isLoading: registerKeyStartLoading } =
    useMutation([MutationKeys.REGISTER_SECURITY_KEY_START], start, {
      onSuccess: async (data, props) => {
        setWaitingForSecurityKey(true);
        const options = parseCreationOptionsFromJSON(data);
        const response = await create(options);
        setWaitingForSecurityKey(false);
        if (response) {
          registerKeyFinish({
            name: props.name,
            rpkc: response.toJSON(),
          });
        } else {
          toaster.error('Failed to get key response, please try again.');
          setModalState({ manageWebAuthNKeysModal: { visible: false } });
        }
      },
    });

  const {
    handleSubmit,
    control,
    formState: { isValid },
    reset,
  } = useForm<FormInputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    defaultValues: {
      name: '',
    },
  });

  const onValidSubmit: SubmitHandler<FormInputs> = (values) => {
    registerKeyStart(values);
  };

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        disabled={
          registerKeyStartLoading ||
          registerKeyFinishLoading ||
          waitingForSecurityKey
        }
        outerLabel="New key name"
      />
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          type="button"
          text="Close"
          onClick={() =>
            setModalState({ manageWebAuthNKeysModal: { visible: false } })
          }
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={
            registerKeyStartLoading ||
            registerKeyFinishLoading ||
            waitingForSecurityKey
          }
          disabled={!isValid}
          text="Add new key"
        />
      </div>
    </form>
  );
};
