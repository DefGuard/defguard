import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import QRCode from 'react-qr-code';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { DelayRender } from '../../../../../../shared/components/layout/DelayRender/DelayRender';
import LoaderSpinner from '../../../../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { IconCopy } from '../../../../../../shared/components/svg';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';

export const RegisterTOTPModal = () => {
  const modalState = useModalStore((state) => state.registerTOTP);
  const setModalsState = useModalStore((state) => state.setState);
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      id="register-totp-modal"
      backdrop
      title={LL.modals.registerTOTP.title()}
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalsState({ registerTOTP: { visible: visibility } })
      }
    >
      <MessageBox type={MessageBoxType.INFO}>
        {parse(LL.modals.registerTOTP.infoMessage())}
      </MessageBox>
      <div className="qr-container">
        <DelayRender delay={1000} fallback={<LoaderSpinner size={250} />}>
          <TOTPRegisterQRCode />
        </DelayRender>
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
  const toaster = useToaster();
  const { LL } = useI18nContext();

  const { data, isLoading } = useQuery([MutationKeys.ENABLE_TOTP_INIT], init, {
    suspense: true,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const qrData = useMemo(
    () => (data ? `otpauth://totp/Defguard?secret=${data.secret}` : undefined),
    [data]
  );

  const handleCopy = () => {
    if (qrData) {
      clipboard
        .write(qrData)
        .then(() => {
          toaster.success(LL.modals.registerTOTP.messages.totpCopied());
        })
        .catch((e) => {
          toaster.error(LL.messages.clipboardError());
          console.error(e);
        });
    }
  };

  if (!qrData || isLoading) return null;

  return (
    <>
      <QRCode value={qrData} size={250} />
      <div className="actions">
        <Button
          icon={<IconCopy />}
          size={ButtonSize.BIG}
          text={LL.modals.registerTOTP.copyPath()}
          onClick={handleCopy}
          loading={isUndefined(qrData)}
        />
      </div>
    </>
  );
};

interface Inputs {
  code: string;
}

const TOTPRegisterForm = () => {
  const toaster = useToaster();
  const {
    auth: {
      mfa: {
        totp: { enable },
      },
    },
  } = useApi();
  const setModalsState = useModalStore((state) => state.setState);
  const queryClient = useQueryClient();
  const { LL, locale } = useI18nContext();
  const schema = useMemo(() => {
    return yup
      .object()
      .shape({
        code: yup
          .string()
          .required(LL.form.error.required())
          .min(6, LL.form.error.minimumLength()),
      })
      .required();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);
  const { mutate, isLoading } = useMutation(
    [MutationKeys.ENABLE_TOTP_FINISH],
    enable,
    {
      onSuccess: (data) => {
        toaster.success(LL.modals.registerTOTP.messages.success());
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        if (data && data.codes) {
          setModalsState({
            recoveryCodesModal: { visible: true, codes: data.codes },
          });
        }
        setModalsState({ registerTOTP: { visible: false } });
      },
      onError: () => {
        setValue('code', '');
        setError('code', {
          message: LL.modals.registerTOTP.form.fields.code.error(),
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
        outerLabel={LL.modals.registerTOTP.form.fields.code.label()}
        autoComplete="one-time-code"
        required
      />
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalsState({ registerTOTP: { visible: false } })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading}
          size={ButtonSize.BIG}
          type="submit"
          text={LL.modals.registerTOTP.form.controls.submit()}
        />
      </div>
    </form>
  );
};
