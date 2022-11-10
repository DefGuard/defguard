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
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../../shared/queries';


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

const toaster = useToaster();

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

  const {
    handleSubmit,
    control,
    formState: { isValid },
    getValues,
    reset,
  } = useForm<FormInputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    defaultValues: {
      name: '',
    },
  });

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const onValidSubmit: SubmitHandler<FormInputs> = (_) => {
    return;
  };

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        disabled={registerKeyFinishLoading || waitingForSecurityKey}
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
          loading={registerKeyFinishLoading || waitingForSecurityKey}
          text="Add new key"
          onClick={async () => {
            if (isValid) {
              setWaitingForSecurityKey(true);
              const formValues = getValues();
              const responseData = await start({
                name: formValues.name,
              }).catch((err) => {
                console.error(err);
                toaster.error(
                  'Error occured while initiating key registration.'
                );
              });
              if (!responseData) {
                setWaitingForSecurityKey(false);
                return;
              }
              const options = parseCreationOptionsFromJSON(responseData);
              //const platform = checkPlatform();
              //if (platform === SupportedPlatform.MAC) {
              //  if (options.publicKey?.authenticatorSelection) {
              //    options.publicKey.authenticatorSelection.authenticatorAttachment =
              //      'platform';
              //  }
              //}
              const response = await create(options);
              setWaitingForSecurityKey(false);
              if (response) {
                registerKeyFinish({
                  name: formValues.name,
                  rpkc: response.toJSON(),
                });
              } else {
                toaster.error('Failed to get key response, please try again.');
                setModalState({ manageWebAuthNKeysModal: { visible: false } });
              }
            }
          }}
        />
      </div>
    </form>
  );
};
