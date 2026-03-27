import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type {
  ApiError,
  LicenseCheckResponse,
} from '../../../../../../../shared/api/types';
import { CopyButton } from '../../../../../../../shared/components/CopyButton/CopyButton';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  openModal,
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
      title={
        modalData?.edit
          ? m.settings_license_key_title()
          : m.settings_license_enter_key_title()
      }
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

type LicenseLimitConflict = {
  label: string;
  current: number;
  limit: number;
};

const sanitizeLicense = (license: string | null | undefined) =>
  license?.replaceAll('\n', '').trim() ?? '';

const getLicenseLimitConflicts = ({
  counts,
  limits,
}: LicenseCheckResponse): LicenseLimitConflict[] => {
  if (!limits) {
    return [];
  }

  const conflicts: LicenseLimitConflict[] = [];

  if (counts.user > limits.users) {
    conflicts.push({
      label: m.cmp_nav_item_users(),
      current: counts.user,
      limit: limits.users,
    });
  }

  if (counts.location > limits.locations) {
    conflicts.push({
      label: m.cmp_nav_item_locations(),
      current: counts.location,
      limit: limits.locations,
    });
  }

  return conflicts;
};

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
      const license = sanitizeLicense(value.license);

      const setLicenseError = () => {
        formApi.setErrorMap({
          onSubmit: {
            fields: {
              license: m.form_error_license(),
            },
          },
        });
      };

      if (license.length > 0) {
        const checkResult = await api
          .checkLicense({ license })
          .catch((e: AxiosError<ApiError>) => {
            const status = e.status ?? e.response?.status;
            if (status && status >= 400 && status < 500) {
              setLicenseError();
            }
            return null;
          });

        if (!checkResult) {
          return;
        }

        const conflicts = getLicenseLimitConflicts(checkResult.data);
        if (conflicts.length > 0) {
          closeModal(modalNameValue);
          openModal(ModalName.LicenseLimitConflict, { conflicts });
          return;
        }
      }

      await patchSettings({
        license,
      }).catch((e: AxiosError<ApiError>) => {
        const status = e.status ?? e.response?.status;
        if (status && status >= 400 && status < 500) {
          setLicenseError();
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
              placeholder={m.settings_license_enter_key_title()}
              label="settings_license_key_title"
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
