import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm } from '../../../../../../../shared/form';
import {
  closeModal,
  openModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { useEffect, useState } from 'react';
import api from '../../../../../../../shared/api/api';
import { getApiErrorMessage } from '../../../../../../../shared/api/apiErrorMessages';
import type { ApiError } from '../../../../../../../shared/api/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useEffectOnce } from '../../../../../../../shared/defguard-ui/hooks/useEffectOnce';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import { useTimer } from '../../../../../../../shared/hooks/useTimer';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

const modalName = ModalName.EmailMfaSetup;
const RESEND_TIMEOUT = 60;

export const EmailMfaSetupModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalName, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalName, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="email-mfa-setup-modal"
      title={m.modal_mfa_enable_email_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
    >
      <ModalContent />
    </Modal>
  );
};

const formSchema = z.object({
  code: z
    .string()
    .trim()
    .min(
      6,
      m.form_error_min_len({
        length: 6,
      }),
    )
    .max(
      6,
      m.form_error_max_len({
        length: 6,
      }),
    ),
});

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  code: '',
};

const ModalContent = () => {
  const user = useUserProfile((s) => s.user);

  const { mutateAsync: enableMfa } = useMutation({
    mutationFn: api.auth.mfa.email.enable,
    meta: {
      invalidate: [['user', user.username]],
    },
    onSuccess: (response) => {
      const recoveryCodes = response.data.codes;
      if (isPresent(recoveryCodes)) {
        openModal(ModalName.RecoveryCodes, recoveryCodes);
      }
      closeModal(modalName);
    },
  });

  const { secondsLeft: resendSecondsLeft, start: startResendTimer } = useTimer();

  const { mutate: resendEmail, isPending: isResending } = useMutation({
    mutationFn: api.auth.mfa.email.init,
    onSuccess: () => {
      startResendTimer(RESEND_TIMEOUT);
    },
    onError: (e: AxiosError<ApiError>) => {
      const code = e.response?.data?.code;
      if (code) {
        Snackbar.error(getApiErrorMessage(code));
      } else {
        Snackbar.error(m.error_unknown());
      }
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onMount: formSchema,
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      await enableMfa(value.code).catch((e: AxiosError<ApiError>) => {
        const errorCode = e.response?.status;
        if (errorCode && errorCode < 500) {
          formApi.setErrorMap({
            onSubmit: {
              fields: {
                code: m.form_error_code(),
              },
            },
          });
        }
      });
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);
  const canSubmit = useStore(form.store, (s) => s.canSubmit);

  useEffectOnce(() => {
    api.auth.mfa.email
      .init()
      .then(() => {
        startResendTimer(RESEND_TIMEOUT);
      })
      .catch((e: AxiosError<ApiError>) => {
        const code = e.response?.data?.code;
        if (code) {
          Snackbar.error(getApiErrorMessage(code));
        } else {
          Snackbar.error(m.error_unknown());
        }
      });
  });

  return (
    <>
      <p>{m.modal_mfa_enable_email_verification()}</p>
      <SizedBox height={ThemeSpacing.Xs} />
      <p>
        {m.modal_mfa_enable_email_content({
          email: user.email,
        })}
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          e.stopPropagation();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="code">
            {(field) => (
              <field.FormInput
                label={m.form_label_verification_code()}
                helper={m.form_helper_verification_code()}
                required
              />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isSubmitting,
          onClick: () => {
            closeModal('emailMfaSetup');
          },
        }}
        submitProps={{
          testId: 'submit',
          text: m.controls_submit(),
          loading: isSubmitting,
          disabled: !canSubmit || isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      >
        <div className="controls-extra">
          <Button
            variant="outlined"
            text={
              resendSecondsLeft > 0
                ? m.modal_mfa_enable_email_resend_countdown({
                    seconds: resendSecondsLeft,
                  })
                : m.modal_mfa_enable_email_resend()
            }
            loading={isResending}
            disabled={resendSecondsLeft > 0}
            onClick={() => {
              form.reset();
              resendEmail();
            }}
          />
        </div>
      </ModalControls>
    </>
  );
};
