import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { ApiError } from '../../../../../../../shared/api/types';
import { CopyButton } from '../../../../../../../shared/components/CopyButton/CopyButton';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';

const modalNameValue = ModalName.SettingsLicense;

type ModalData = {
  edit: boolean;
  license: string | null;
};

export const SettingsLicenseModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, ({ license }) => {
      setModalData({
        edit: isPresent(license) && license.length > 0,
        license: license ?? null,
      });
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
      title={modalData?.edit ? 'License key' : 'Enter license key'}
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
  license: z.string().trim().nullable(),
});

type FormFields = z.infer<typeof formSchema>;

const ModalContent = ({ license: initialLicense }: ModalData) => {
  const defaultValues: FormFields = useMemo(
    () => ({
      license: initialLicense ?? '',
    }),
    [initialLicense],
  );

  const { mutateAsync: patchSettings } = useMutation({
    mutationFn: api.settings.patchSettings,
    onSuccess: () => {
      closeModal(modalNameValue);
    },
    meta: {
      invalidate: [['settings'], ['enterprise_info']],
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      await patchSettings({
        license: value.license?.replaceAll('\n', '').trim() ?? '',
      }).catch((e: AxiosError<ApiError>) => {
        if (e.status && e.status >= 400 && e.status < 500) {
          formApi.setErrorMap({
            onSubmit: {
              fields: {
                license: m.form_error_license(),
              },
            },
          });
        }
      });
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
        <form.AppField name="license">
          {(field) => (
            <field.FormTextarea
              placeholder="Enter license key"
              label="License key"
              maxHeight={300}
            />
          )}
        </form.AppField>
        <ModalControls
          cancelProps={{
            disabled: isSubmitting,
            text: m.controls_cancel(),
            onClick: () => {
              closeModal(modalNameValue);
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
          {isPresent(initialLicense) && initialLicense.length > 0 && (
            <CopyButton value={initialLicense} />
          )}
        </ModalControls>
      </form.AppForm>
    </form>
  );
};
