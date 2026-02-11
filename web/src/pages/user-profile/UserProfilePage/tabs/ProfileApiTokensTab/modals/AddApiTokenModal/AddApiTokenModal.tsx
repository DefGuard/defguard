import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import { CopyField } from '../../../../../../../shared/defguard-ui/components/CopyField/CopyField';
import { IconKind } from '../../../../../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAddApiTokenModal } from '../../../../../../../shared/hooks/modalControls/types';

const modalNameKey = ModalName.AddApiToken;

export const AddApiTokenModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenAddApiTokenModal | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
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
      id="add-api-token-modal"
      title={m.modal_add_api_title()}
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
  name: z.string().trim().min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  name: '',
};

const ModalContent = ({ username }: OpenAddApiTokenModal) => {
  const [token, setToken] = useState<string | null>(null);

  const { mutateAsync } = useMutation({
    mutationFn: api.user.addApiToken,
    meta: {
      invalidate: [['user-overview'], ['user', username, 'api_token']],
    },
    onSuccess: (response) => {
      setToken(response.data.token);
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
      await mutateAsync({
        name: value.name,
        username,
      });
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  if (isPresent(token)) {
    return (
      <>
        <InfoBanner
          icon={IconKind.WarningOutlined}
          variant="warning"
          text={m.modal_add_api_token_copy_warning()}
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <CopyField
          copyTooltip={m.misc_clipboard_copy()}
          text={token}
          data-testid="copy-field"
          label={m.modal_add_api_token_copy_copy_label()}
        />
        <ModalControls
          submitProps={{
            testId: 'close',
            text: m.controls_close(),
            onClick: () => {
              closeModal(modalNameKey);
            },
          }}
        />
      </>
    );
  }

  return (
    <>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.form_label_name()} />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          testId: 'cancel',
          disabled: isSubmitting,
          text: m.controls_cancel(),
          onClick: () => {
            closeModal(modalNameKey);
          },
        }}
        submitProps={{
          testId: 'submit',
          text: m.controls_submit(),
          loading: isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
