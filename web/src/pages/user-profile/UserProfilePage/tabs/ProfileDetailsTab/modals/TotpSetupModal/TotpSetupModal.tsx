import { useEffect, useMemo, useState } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { QRCodeCanvas } from 'qrcode.react';
import type z from 'zod';
import api from '../../../../../../../shared/api/api';
import type { ApiError } from '../../../../../../../shared/api/types';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { CopyField } from '../../../../../../../shared/defguard-ui/components/CopyField/CopyField';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { createZodIssue } from '../../../../../../../shared/defguard-ui/utils/zod';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  openModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { totpCodeFormSchema } from '../../../../../../../shared/schema/totpCode';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

const modalName = ModalName.TotpSetup;

export const TotpSetupModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const sub = subscribeOpenModal(modalName, () => {
      setOpen(true);
    });
    const close = subscribeCloseModal(modalName, () => setOpen(false));
    return () => {
      sub.unsubscribe();
      close.unsubscribe();
    };
  }, []);

  return (
    <Modal
      size="small"
      title={m.modal_mfa_enable_totp_title()}
      id="totp-setup-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
    >
      <ModalContent />
    </Modal>
  );
};

const formSchema = totpCodeFormSchema;

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  code: '',
};

const ModalContent = () => {
  const username = useUserProfile((s) => s.user.username);
  const { mutateAsync: enableTotp } = useMutation({
    mutationFn: api.auth.mfa.totp.enable,
    meta: {
      invalidate: [['user', username]],
    },
    onSuccess: (response) => {
      if (response.data.codes) {
        closeModal(modalName);
        openModal(ModalName.RecoveryCodes, response.data.codes);
      } else {
        closeModal(modalName);
      }
    },
  });

  const { data: totpInitResponse } = useQuery({
    queryFn: api.auth.mfa.totp.init,
    queryKey: ['totp_setup_init'],
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const qrData = useMemo(() => {
    if (totpInitResponse) {
      return `otpauth://totp/Defguard?secret=${totpInitResponse.data.secret}`;
    }
    return null;
  }, [totpInitResponse]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onChange: formSchema,
      onSubmit: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      await enableTotp(value.code).catch((e: AxiosError<ApiError>) => {
        if (e.response?.data.msg === 'Invalid TOTP code' || e.code === '404') {
          formApi.setErrorMap({
            onSubmit: {
              fields: {
                code: createZodIssue(m.form_error_code(), ['code']),
              },
            },
          });
        }
      });
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <section>
        <header>
          <Badge
            variant="success"
            text={m.state_step({
              step: 1,
            })}
          />
          <p>{m.modal_mfa_enable_totp_step_1_title()}</p>
        </header>
        <SizedBox height={ThemeSpacing.Sm} />
        <p className="step-1-content">{m.modal_mfa_enable_totp_step_1_content()}</p>
        <SizedBox height={ThemeSpacing.Xl2} />
        {isPresent(qrData) && isPresent(totpInitResponse) && (
          <div className="qr">
            <QRCodeCanvas value={qrData} size={130} />
            <SizedBox height={ThemeSpacing.Xl} />
            <p>{m.modal_mfa_enable_totp_qr_problem()}</p>
            <SizedBox height={ThemeSpacing.Sm} />
            <CopyField
              text={totpInitResponse.data.secret}
              copyTooltip={m.misc_clipboard_copy()}
            />
          </div>
        )}
      </section>
      <Divider orientation="horizontal" />
      <section>
        <header>
          <Badge
            text={m.state_step({
              step: 2,
            })}
            variant="success"
          />
          <p>{m.modal_mfa_enable_totp_step_2_title()}</p>
        </header>
        <SizedBox height={ThemeSpacing.Xl} />
        <form
          onSubmit={(e) => {
            e.preventDefault();
            e.stopPropagation();
            form.handleSubmit();
          }}
        >
          <form.AppForm>
            <form.AppField name="code">
              {(field) => <field.FormInput label={m.form_label_auth_code()} required />}
            </form.AppField>
          </form.AppForm>
        </form>
      </section>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isSubmitting,
          onClick: () => {
            closeModal(modalName);
          },
        }}
        submitProps={{
          loading: isSubmitting,
          text: m.controls_verify_code(),
          onClick: () => form.handleSubmit(),
        }}
      />
    </>
  );
};
