import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm } from '../../../../../../../shared/defguard-ui/form';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import type { ModalNameValue } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { useEffect, useState } from 'react';
import api from '../../../../../../../shared/api/api';
import type { ApiError } from '../../../../../../../shared/api/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { createZodIssue } from '../../../../../../../shared/defguard-ui/utils/zod';
import { formChangeLogic } from '../../../../../../../shared/form';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

const modalNameKey: ModalNameValue = 'emailMfaSetup' as const;

export const EmailMfaSetupModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
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
  const user = useUserProfile((s) => s.profile.user);

  const { mutateAsync: enableMfa } = useMutation({
    mutationFn: api.auth.mfa.email.enable,
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      await enableMfa(value.code).catch((e: AxiosError<ApiError>) => {
        if (e.code && parseInt(e.code, 2) < 500) {
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

  useEffect(() => {
    void api.auth.mfa.email.init();
  }, []);

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
              <field.FormInput label={m.form_label_verification_code()} required />
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
          text: m.controls_submit(),
          loading: isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      >
        <div className="controls-extra">
          <Button
            variant="outlined"
            text={m.modal_mfa_enable_email_resend()}
            onClick={() => {}}
          />
        </div>
      </ModalControls>
    </>
  );
};
