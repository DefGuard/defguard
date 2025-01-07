import {
  create,
  parseCreationOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useModalStore } from '../../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../../shared/queries';

interface FormInputs {
  name: string;
}

export const RegisterWebAuthNForm = () => {
  const { LL } = useI18nContext();
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

  const { mutate: registerKeyFinish, isPending: registerKeyFinishLoading } = useMutation({
    mutationKey: [MutationKeys.REGISTER_SECURITY_KEY_FINISH],
    mutationFn: finish,
    onSuccess: (data) => {
      toaster.success(LL.modals.manageWebAuthNKeys.form.messages.success());
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_USER_PROFILE],
      });
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
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .min(1, LL.form.error.required())
          .min(4, LL.form.error.minimumLength()),
      }),
    [LL.form.error],
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
    getValues,
    reset,
  } = useForm<FormInputs>({
    resolver: zodResolver(zodSchema),
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
    <form onSubmit={void handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        disabled={registerKeyFinishLoading || waitingForSecurityKey}
        label={LL.modals.manageWebAuthNKeys.form.fields.name.label()}
      />
      <div className="controls">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          type="button"
          text={LL.form.close()}
          onClick={() => setModalState({ manageWebAuthNKeysModal: { visible: false } })}
        />
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={registerKeyFinishLoading || waitingForSecurityKey}
          text={LL.modals.manageWebAuthNKeys.form.controls.submit()}
          // eslint-disable-next-line @typescript-eslint/no-misused-promises
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
                let errorType: string;
                const split = String(err).split(':');
                if (split.length > 1) {
                  errorType = split[0];
                  if (errorType === 'InvalidStateError') {
                    toaster.error(
                      LL.modals.manageWebAuthNKeys.messages.duplicateKeyError(),
                    );
                  }
                } else {
                  toaster.error(LL.messages.error());
                }
                return null;
              });
              setWaitingForSecurityKey(false);
              if (!response) {
                return;
              }
              if (response) {
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
