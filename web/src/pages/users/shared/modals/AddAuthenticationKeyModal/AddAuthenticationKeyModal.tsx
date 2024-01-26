import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm, useWatch } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconAuthenticationKey from '../../../../../shared/components/svg/IconAuthenticationKey';
import SvgIconCheckmark from '../../../../../shared/components/svg/IconCheckmark';
import { ColorsRGB } from '../../../../../shared/constants';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useTheme } from '../../../../../shared/defguard-ui/hooks/theme/useTheme';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { AuthenticationKeyType } from '../../../../../shared/types';
import { AuthenticationKeyFormTextField } from './AuthenticationKeyFormTextField';

interface FormValues {
  name: string;
  key_type: string;
  key: string;
}

export const AddAuthenticationKeyModal = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();

  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addAuthenticationKeyModal, state.setAddAuthenticationKeyModal],
    shallow,
  );

  const {
    user: { addAuthenticationKey },
  } = useApi();

  const queryClient = useQueryClient();
  const { colors } = useTheme();

  const {
    mutate: addAuthenticationKeyMutation,
    isLoading: isAddAuthenticationKeyLoading,
  } = useMutation(addAuthenticationKey, {
    onSuccess: () => {
      setModalState({ visible: false });
      reset();
      queryClient.invalidateQueries([QueryKeys.FETCH_AUTHENTICATION_KEYS]);
      toaster.success(LL.userPage.authenticationKeys.addModal.messages.keyAdded());
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onError: (error: any) => {
      console.error(error);

      if (error.code === 'ERR_BAD_REQUEST') {
        if (error.response.data.msg === 'invalid key format') {
          setError('key', {
            message:
              LL.userPage.authenticationKeys.addModal.messages.unsupportedKeyFormat(),
            type: 'value',
          });
          return;
        }

        if (error.response.data.msg === 'key already exists') {
          setError('key', {
            message: LL.userPage.authenticationKeys.addModal.messages.keyExists(),
            type: 'value',
          });
          return;
        }
      }

      toaster.error(LL.userPage.authenticationKeys.addModal.messages.genericError());
    },
  });

  const schema = useMemo(
    () =>
      yup
        .object()
        .shape({
          name: yup.string().required(LL.form.error.required()),
          key: yup.string().required(LL.form.error.required()),
          key_type: yup.string().required(LL.form.error.required()),
        })
        .required(),
    [LL],
  );

  const submitHandler: SubmitHandler<FormValues> = async (values) => {
    const data = {
      key_type: values.key_type,
      name: values.name.trim(),
      key: values.key.trim(),
    } as FormValues;
    addAuthenticationKeyMutation(data);
  };

  const { handleSubmit, control, reset, setValue, setError } = useForm<FormValues>({
    defaultValues: {
      name: '',
      key_type: AuthenticationKeyType.SSH,
      key: '',
    },
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const keyTypeValue = useWatch({ control, name: 'key_type' });

  return (
    <ModalWithTitle
      id="add-authentication-key-modal"
      title={LL.userPage.authenticationKeys.addModal.header()}
      isOpen={isOpen}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      onClose={() => {
        reset();
      }}
      backdrop
    >
      <div className="add-authentication-key-content">
        <div className="authentication-keys-container">
          <Button
            styleVariant={
              keyTypeValue === AuthenticationKeyType.SSH
                ? ButtonStyleVariant.PRIMARY
                : ButtonStyleVariant.STANDARD
            }
            size={ButtonSize.SMALL}
            text={AuthenticationKeyType.SSH}
            onClick={() => {
              setValue('key_type', AuthenticationKeyType.SSH);
            }}
            icon={
              <IconAuthenticationKey
                fill={
                  keyTypeValue === AuthenticationKeyType.SSH
                    ? ColorsRGB.White
                    : colors.textBodyTertiary
                }
              />
            }
          />
          <Button
            styleVariant={
              keyTypeValue === AuthenticationKeyType.GPG
                ? ButtonStyleVariant.PRIMARY
                : ButtonStyleVariant.STANDARD
            }
            size={ButtonSize.SMALL}
            text={AuthenticationKeyType.GPG}
            onClick={() => {
              setValue('key_type', AuthenticationKeyType.GPG);
            }}
            icon={
              <IconAuthenticationKey
                fill={
                  keyTypeValue === AuthenticationKeyType.GPG
                    ? ColorsRGB.White
                    : colors.textBodyTertiary
                }
              />
            }
          />
        </div>
        <form onSubmit={handleSubmit(submitHandler)}>
          <FormInput
            label={LL.userPage.authenticationKeys.addModal.keyNameLabel()}
            placeholder={LL.userPage.authenticationKeys.addModal.keyNamePlaceholder()}
            controller={{ control, name: 'name' }}
          />

          <Label style={{ marginBottom: 10 }}>
            {LL.userPage.authenticationKeys.addModal.keyLabel()}
          </Label>
          <AuthenticationKeyFormTextField
            data-testid="authentication-key-value"
            placeholder={
              keyTypeValue === AuthenticationKeyType.SSH
                ? LL.userPage.authenticationKeys.addModal.sshKeyPlaceholder()
                : LL.userPage.authenticationKeys.addModal.gpgKeyPlaceholder()
            }
            controller={{ control, name: 'key' }}
          />
          <div className="add-authentication-key-buttons-container">
            <Button
              data-testid="submit-add-authentication-key"
              type="submit"
              styleVariant={ButtonStyleVariant.PRIMARY}
              icon={isAddAuthenticationKeyLoading ? null : <SvgIconCheckmark />}
              text={
                isAddAuthenticationKeyLoading
                  ? ''
                  : LL.userPage.authenticationKeys.addKey()
              }
              loading={isAddAuthenticationKeyLoading}
            />
            <Button
              text={LL.common.controls.cancel()}
              styleVariant={ButtonStyleVariant.STANDARD}
              onClick={() => {
                setModalState({ visible: false });
                reset();
              }}
            />
          </div>
        </form>
      </div>
    </ModalWithTitle>
  );
};
