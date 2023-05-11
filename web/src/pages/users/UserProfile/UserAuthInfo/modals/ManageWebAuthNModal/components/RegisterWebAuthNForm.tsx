import {
  create,
  parseCreationOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
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

export const RegisterWebAuthNForm = () => {
  const { LL, locale } = useI18nContext();
  const toaster = useToaster();
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

  const { mutate: registerKeyFinish, isLoading: registerKeyFinishLoading } = useMutation(
    [MutationKeys.REGISTER_SECURITY_KEY_FINISH],
    finish,
    {
      onSuccess: (data) => {
        toaster.success(LL.modals.manageWebAuthNKeys.form.messages.success());
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        reset();
        if (data && data.codes) {
          setModalState({
            recoveryCodesModal: { visible: true, codes: data.codes },
            manageWebAuthNKeysModal: { visible: false },
          });
        }
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );

  const formSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
        })
        .required(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [locale]
  );

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
        outerLabel={LL.modals.manageWebAuthNKeys.form.fields.name.label()}
      />
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          type="button"
          text={LL.form.close()}
          onClick={() => setModalState({ manageWebAuthNKeysModal: { visible: false } })}
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={registerKeyFinishLoading || waitingForSecurityKey}
          text={LL.modals.manageWebAuthNKeys.form.controls.submit()}
          onClick={async () => {
            if (isValid) {
              setWaitingForSecurityKey(true);
              const formValues = getValues();
              const responseData = await start({
                name: formValues.name,
              }).catch((err) => {
                toaster.error(LL.messages.error());
                console.error(err);
              });
              if (!responseData) {
                setWaitingForSecurityKey(false);
                return;
              }
              const options = parseCreationOptionsFromJSON(responseData);
              const response = await create(options).catch((err) => {
                console.error(err);
                return null;
              });
              setWaitingForSecurityKey(false);
              if (!response) {
                toaster.error(LL.messages.error());
                return;
              }
              if (response) {
                console.log(response);
                registerKeyFinish({
                  name: formValues.name,
                  rpkc: response.toJSON(),
                });
              } else {
                toaster.error(LL.messages.error());
                setModalState({ manageWebAuthNKeysModal: { visible: false } });
              }
            }
          }}
        />
      </div>
    </form>
  );
};
