import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenEditNetworkDeviceModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import api from '../../../../shared/api/api';
import { Select } from '../../../../shared/defguard-ui/components/Select/Select';
import type { SelectSingleValue } from '../../../../shared/defguard-ui/components/Select/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';

const modalNameValue = ModalName.EditNetworkDevice;

type ModalData = OpenEditNetworkDeviceModal;

export const EditNetworkDeviceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

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
      id="edit-network-device-modal"
      title={'Edit network device'}
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

const ModalContent = ({ device, reservedNames }: ModalData) => {
  const { mutateAsync: editDevice } = useMutation({
    mutationFn: api.network_device.editDevice,
    meta: {
      invalidate: ['device', 'network'],
    },
    onSuccess: () => {
      closeModal(modalNameValue);
    },
  });
  const locationOption: SelectSingleValue<number> = {
    key: device.location.id,
    label: device.location.name,
    value: device.location.id,
  };

  const formSchema = useMemo(
    () =>
      z.object({
        name: z
          .string(m.form_error_required())
          .trim()
          .min(1, m.form_error_required())
          .refine((value) => {
            if (value === device.name) return true;
            return !reservedNames.includes(value);
          }, m.form_error_name_reserved()),
        description: z.string().trim().nullable(),
        modifiableIpParts: z.array(
          z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
        ),
      }),
    [device.name, reservedNames],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues: FormFields = useMemo(
    () => ({
      name: device.name,
      description: device.description ?? null,
      modifiableIpParts: device.split_ips.map((item) => item.modifiable_part),
    }),
    [device],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
      onSubmitAsync: async ({ value }) => {
        const errors: Record<`modifiableIpParts[${number}]`, string> = {};
        const formValues = value.modifiableIpParts.map(
          (modifiedPart, index) => device.split_ips[index].network_part + modifiedPart,
        );
        const { data: validationResponse } = await api.network_device.validateIps({
          ips: formValues,
          locationId: device.location.id,
        });
        validationResponse.forEach(({ valid, available }, index) => {
          if (!valid) {
            errors[`modifiableIpParts[${index}]`] = m.form_error_ip_invalid();
          }
          if (!available) {
            errors[`modifiableIpParts[${index}]`] = m.form_error_ip_reserved();
          }
        });
        if (Object.keys(errors).length) {
          return {
            fields: errors,
          };
        }
        return null;
      },
    },
    onSubmit: async ({ value }) => {
      const assignedIps = value.modifiableIpParts.map(
        (part, index) => device.split_ips[index].network_part + part,
      );
      await editDevice({
        id: device.id,
        assigned_ips: assignedIps,
        name: device.name,
        description: device.description,
      });
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <Select
          label="Location"
          value={locationOption}
          options={[locationOption]}
          onChange={() => {}}
          required
          disabled
        />
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.form_label_device_name()} />}
          </form.AppField>
          <form.AppField name="description">
            {(field) => <field.FormInput required label={m.form_label_description()} />}
          </form.AppField>
          <form.AppField name="modifiableIpParts" mode="array">
            {(field) =>
              field.state.value.map((_, index) => (
                <form.AppField key={index} name={`modifiableIpParts[${index}]`}>
                  {(subField) => (
                    <subField.FormSuggestedIPInput
                      data={device.split_ips[index]}
                      label="Assigned IP Address"
                      required
                    />
                  )}
                </form.AppField>
              ))
            }
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          disabled: isSubmitting,
          text: m.controls_cancel(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
        submitProps={{
          text: m.controls_save_changes(),
          loading: isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
