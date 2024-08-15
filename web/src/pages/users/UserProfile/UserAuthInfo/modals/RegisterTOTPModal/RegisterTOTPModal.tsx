import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import QRCode from 'react-qr-code';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCopy from '../../../../../../shared/components/svg/IconCopy';
import { DelayRender } from '../../../../../../shared/components/utils/DelayRender/DelayRender';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { trimObjectStrings } from '../../../../../../shared/utils/trimObjectStrings';

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
  const { writeToClipboard } = useClipboard();
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
    [data],
  );

  const handleCopy = () => {
    if (data && data.secret) {
      writeToClipboard(data.secret, LL.modals.registerTOTP.messages.totpCopied());
    }
  };

  if (!qrData || isLoading) return null;

  return (
    <>
      <QRCode value={qrData} size={250} />
      <div className="actions">
        <Button
          data-testid="copy-totp"
          icon={<IconCopy />}
          size={ButtonSize.LARGE}
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
  const { LL } = useI18nContext();
  const zodSchema = useMemo(
    () =>
      z.object({
        code: z.string().min(6, LL.form.error.minimumLength()),
      }),
    [LL.form.error],
  );
  const { mutate, isLoading } = useMutation([MutationKeys.ENABLE_TOTP_FINISH], enable, {
    onSuccess: (data) => {
      toaster.success(LL.modals.registerTOTP.messages.success());
      queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
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
  });
  const { handleSubmit, control, setError, setValue } = useForm({
    resolver: zodResolver(zodSchema),
    mode: 'all',
    defaultValues: {
      code: '',
    },
  });
  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    values = trimObjectStrings(values);
    mutate({
      code: String(values.code),
    });
  };
  return (
    <form data-testid="register-totp-form" onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'code' }}
        label={LL.modals.registerTOTP.form.fields.code.label()}
        autoComplete="one-time-code"
        required
      />
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalsState({ registerTOTP: { visible: false } })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading}
          size={ButtonSize.LARGE}
          type="submit"
          text={LL.modals.registerTOTP.form.controls.submit()}
        />
      </div>
    </form>
  );
};
