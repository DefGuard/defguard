import { useMutation } from '@tanstack/react-query';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenCEWebhookModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useCallback, useEffect, useMemo, useState } from 'react';
import z from 'zod';
import api from '../../../../shared/api/api';
import { DescriptionBlock } from '../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';

const modalNameValue = ModalName.CEWebhook;

type ModalData = OpenCEWebhookModal;

export const CeWebhookModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);
  const isEdit = isPresent(modalData?.webhook);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="ce-webhook-modal"
      title={isEdit ? m.modal_ce_webhook_edit_title() : m.modal_ce_webhook_create_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const formSchema = z.object({
  url: z.url(m.form_error_invalid()).min(1, m.form_error_required()),
  description: z
    .string()
    .min(1, m.form_error_required())
    .min(
      4,
      m.form_error_min_len({
        length: 4,
      }),
    )
    .max(
      65,
      m.form_error_max_len({
        length: 65,
      }),
    ),
  token: z
    .string()
    .min(1, m.form_error_required())
    .max(
      250,
      m.form_error_max_len({
        length: 250,
      }),
    ),
  enabled: z.boolean(),
  on_user_created: z.boolean(),
  on_user_deleted: z.boolean(),
  on_user_modified: z.boolean(),
  on_hwkey_provision: z.boolean(),
});

type FormFields = z.infer<typeof formSchema>;

const ModalContent = ({ webhook }: ModalData) => {
  const isEdit = isPresent(webhook);

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(webhook)) {
      return webhook;
    }
    return {
      description: '',
      enabled: true,
      on_hwkey_provision: false,
      on_user_created: false,
      on_user_deleted: false,
      on_user_modified: false,
      token: '',
      url: '',
    };
  }, [webhook]);
  const onSuccess = useCallback(() => {
    closeModal(modalNameValue);
  }, []);

  const { mutateAsync: addWebhook } = useMutation({
    mutationFn: api.webhook.addWebhook,
    onSuccess,
    meta: {
      invalidate: ['webhook'],
    },
  });

  const { mutateAsync: editWebhook } = useMutation({
    mutationFn: api.webhook.editWebhook,
    onSuccess,
    meta: {
      invalidate: ['webhook'],
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      if (isEdit) {
        await editWebhook({
          id: webhook.id,
          ...value,
        });
      } else {
        await addWebhook(value);
      }
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <form.AppField name="url">
          {(field) => <field.FormInput required label={m.form_label_webhook_url()} />}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="description">
          {(field) => <field.FormInput required label={m.form_label_description()} />}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="token">
          {(field) => (
            <field.FormInput
              required
              label={m.form_label_secret_token()}
              type="password"
            />
          )}
        </form.AppField>
        <Divider spacing={ThemeSpacing.Xl} />
        <DescriptionBlock title={m.modal_ce_webhook_events_title()}>
          <p>{m.test_placeholder_long()}</p>
        </DescriptionBlock>
        <SizedBox height={ThemeSpacing.Xl} />
        <div className="webhooks">
          <form.AppField name="on_user_created">
            {(field) => <field.FormCheckbox text={m.cmp_webhook_event_user_add()} />}
          </form.AppField>
          <form.AppField name="on_user_modified">
            {(field) => <field.FormCheckbox text={m.cmp_webhook_event_user_edit()} />}
          </form.AppField>
          <form.AppField name="on_user_deleted">
            {(field) => <field.FormCheckbox text={m.cmp_webhook_event_user_delete()} />}
          </form.AppField>
          <form.AppField name="on_hwkey_provision">
            {(field) => <field.FormCheckbox text={m.cmp_webhook_event_user_hw()} />}
          </form.AppField>
        </div>
        <ModalControls
          cancelProps={{
            disabled: isSubmitting,
            text: m.controls_cancel(),
            onClick: () => {
              closeModal(modalNameValue);
            },
          }}
          submitProps={{
            testId: 'submit',
            text: isEdit ? m.controls_save_changes() : m.controls_submit(),
            loading: isSubmitting,
            onClick: () => {
              form.handleSubmit();
            },
          }}
        >
          <form.AppField name="enabled">
            {(field) => <field.FormToggle label={m.state_enabled()} />}
          </form.AppField>
        </ModalControls>
      </form.AppForm>
    </form>
  );
};
