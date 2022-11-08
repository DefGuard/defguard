import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { QrCode } from '../../../../../../shared/components/layout/QrCode/QrCode';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { toaster } from '../../../../../../shared/utils/toaster';

export const RegisterTOTPModal = () => {
  const modalState = useModalStore((state) => state.registerTOTP);
  const setModalsState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      id="register-totp-modal"
      backdrop
      title="Authenticator App Setup"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalsState({ registerTOTP: { visible: visibility } })
      }
    >
      <MessageBox type={MessageBoxType.INFO}>
        <p>
          To setup your MFA, scan this QR code with your authenticator app, then
          enter the code in the field below:
        </p>
      </MessageBox>
      <div className="qr-container">
        <TOTPRegisterQRCode />
      </div>
      <TOTPRegisterForm />
    </ModalWithTitle>
  );
};

const TOTPRegisterQRCode = () => {
  const {
    auth: {
      mfa: {
        totp: { init },
      },
    },
  } = useApi();

  const { data, isLoading, mutate } = useMutation(
    [MutationKeys.ENABLE_TOTP_INIT],
    init,
    {
      onError: (err) => {
        console.error(err);
        toaster.error('TOTP Initialization failed');
      },
    }
  );

  useEffect(() => {
    if (!isLoading && !data) {
      mutate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const qrData = useMemo(
    () => (data ? `otpauth://totp/Defguard?secret=${data.secret}` : undefined),
    [data]
  );

  return <>{qrData && <QrCode data={qrData} />}</>;
};

interface Inputs {
  code: string;
}

const schema = yup
  .object()
  .shape({
    code: yup.string().required('Code is required').min(6, 'Code is to short'),
  })
  .required();

const TOTPRegisterForm = () => {
  const {
    auth: {
      mfa: {
        totp: { enable },
      },
    },
  } = useApi();
  const setModalsState = useModalStore((state) => state.setState);
  const queryClient = useQueryClient();
  const { mutate, isLoading } = useMutation(
    [MutationKeys.ENABLE_TOTP_FINISH],
    enable,
    {
      onSuccess: (data) => {
        console.log(data);
        toaster.success('TOTP Enabled');
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        setModalsState({ registerTOTP: { visible: false } });
      },
      onError: () => {
        toaster.error('Provided code is invalid');
        setValue('code', '');
        setError('code', {
          message: 'Code is invalid',
        });
      },
    }
  );
  const { handleSubmit, control, setError, setValue } = useForm({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      code: '',
    },
  });
  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    mutate({
      code: Number(values.code),
    });
  };
  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'code' }}
        outerLabel="Authenticator code"
      />
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={() => setModalsState({ registerTOTP: { visible: false } })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading}
          size={ButtonSize.BIG}
          type="submit"
          text="Verify code"
        />
      </div>
    </form>
  );
};
