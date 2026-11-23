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
import type { NetworkDevice } from '../../../../shared/api/types';
import { DescriptionBlock } from '../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { CopyField } from '../../../../shared/defguard-ui/components/CopyField/CopyField';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Select } from '../../../../shared/defguard-ui/components/Select/Select';
import type { SelectSingleValue } from '../../../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { generatePresharedKey } from '../../../../shared/utils/generatePresharedKey';
import { refinePresharedKey } from '../../components/PresharedKeyFields/PresharedKeyFields';

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
      title={m.modal_edit_network_device_title()}
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

type PresharedKeyFormValues = {
  usePresharedKey: boolean;
  generatePresharedKey: boolean;
  preshared_key?: string | null;
};

const isPresharedKeyChanged = (
  device: NetworkDevice,
  values: PresharedKeyFormValues,
): boolean => {
  if (!values.usePresharedKey) return false;
  const keyTyped = (values.preshared_key?.trim().length ?? 0) > 0;
  return values.generatePresharedKey || !device.preshared_key_set || keyTyped;
};

const presharedKeyRequestValue = (
  device: NetworkDevice,
  values: PresharedKeyFormValues,
): string | null | undefined => {
  if (!values.usePresharedKey) {
    return device.preshared_key_set ? null : undefined;
  }
  if (!isPresharedKeyChanged(device, values)) return undefined;
  return values.generatePresharedKey
    ? generatePresharedKey()
    : (values.preshared_key ?? undefined);
};

const ModalContent = ({ device, reservedNames }: ModalData) => {
  const [generatedPresharedKey, setGeneratedPresharedKey] = useState<string | null>(null);
  const { mutateAsync: editDevice } = useMutation({
    mutationFn: api.network_device.editDevice,
    meta: {
      invalidate: [['device', 'network'], ['network']],
    },
  });
  const locationOption: SelectSingleValue<number> = {
    key: device.location.id,
    label: device.location.name,
    value: device.location.id,
  };

  const formSchema = useMemo(
    () =>
      z
        .object({
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
          usePresharedKey: z.boolean(),
          generatePresharedKey: z.boolean(),
          preshared_key: z.string().trim().nullable().optional(),
        })
        .superRefine((values, ctx) => {
          if (isPresharedKeyChanged(device, values)) {
            refinePresharedKey(values, ctx);
          }
        }),
    [device, reservedNames],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues: FormFields = useMemo(
    () => ({
      name: device.name,
      description: device.description ?? null,
      modifiableIpParts: device.split_ips.map((item) => item.modifiable_part),
      usePresharedKey: device.preshared_key_set,
      generatePresharedKey: false,
      preshared_key: device.preshared_key_set ? undefined : null,
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
          deviceId: device.id,
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
      const presharedKey = presharedKeyRequestValue(device, value);
      await editDevice({
        id: device.id,
        assigned_ips: assignedIps,
        name: value.name,
        description: value.description ?? undefined,
        preshared_key: presharedKey,
      });
      if (isPresharedKeyChanged(device, value) && value.generatePresharedKey) {
        setGeneratedPresharedKey(presharedKey ?? null);
      } else {
        closeModal(modalNameValue);
      }
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  if (isPresent(generatedPresharedKey)) {
    return <GeneratedPresharedKeyStep presharedKey={generatedPresharedKey} />;
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
          <EvenSplit parts={2}>
            <Select
              label={m.form_label_location()}
              helper={m.form_helper_location()}
              value={locationOption}
              options={[locationOption]}
              onChange={() => {}}
              required
              disabled
            />
            <form.AppField name="name">
              {(field) => (
                <field.FormInput
                  required
                  label={m.form_label_device_name()}
                  helper={m.form_helper_device_name()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <form.AppField name="description">
            {(field) => (
              <field.FormInput
                label={m.form_label_description()}
                helper={m.form_helper_description()}
              />
            )}
          </form.AppField>
          <form.AppField name="modifiableIpParts" mode="array">
            {(field) =>
              field.state.value.map((_, index) => (
                <form.AppField key={index} name={`modifiableIpParts[${index}]`}>
                  {(subField) => (
                    <subField.FormSuggestedIPInput
                      data={device.split_ips[index]}
                      label={m.form_label_assigned_ip_address()}
                      helper={m.form_helper_assigned_ip_address()}
                      required
                    />
                  )}
                </form.AppField>
              ))
            }
          </form.AppField>
          <form.AppField name="usePresharedKey">
            {(field) => (
              <field.FormInteractiveBlock
                variant="toggle"
                data-testid="field-usePresharedKey"
                title={m.modal_network_device_psk_title()}
                content={m.modal_network_device_psk_content()}
              >
                <form.Subscribe
                  selector={(s) => ({
                    usePresharedKey: s.values.usePresharedKey,
                    generatePresharedKey: s.values.generatePresharedKey,
                    keyChanged: isPresharedKeyChanged(device, s.values),
                  })}
                >
                  {({ usePresharedKey, generatePresharedKey, keyChanged }) => (
                    <Fold open={usePresharedKey}>
                      <SizedBox height={ThemeSpacing.Lg} />
                      <form.AppField name="preshared_key">
                        {(field) => (
                          <field.FormSecretInput
                            required
                            stored={device.preshared_key_set}
                            disabled={generatePresharedKey}
                            label={m.form_label_preshared_key()}
                          />
                        )}
                      </form.AppField>
                      <SizedBox height={ThemeSpacing.Lg} />
                      <form.AppField name="generatePresharedKey">
                        {(field) => (
                          <field.FormCheckbox
                            text={m.modal_network_device_psk_generate_new()}
                          />
                        )}
                      </form.AppField>
                      <Fold open={keyChanged}>
                        <SizedBox height={ThemeSpacing.Xl} />
                        <InfoBanner
                          variant="warning"
                          icon={IconKind.WarningOutlined}
                          text={m.modal_network_device_psk_edit_warning()}
                        />
                      </Fold>
                    </Fold>
                  )}
                </form.Subscribe>
              </field.FormInteractiveBlock>
            )}
          </form.AppField>
          {device.preshared_key_set && (
            <form.Subscribe selector={(s) => s.values.usePresharedKey}>
              {(usePresharedKey) =>
                !usePresharedKey && (
                  <InfoBanner
                    variant="warning"
                    icon={IconKind.WarningOutlined}
                    text={m.modal_network_device_psk_remove_warning()}
                  />
                )
              }
            </form.Subscribe>
          )}
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

type Props = {
  presharedKey: string;
};

const GeneratedPresharedKeyStep = ({ presharedKey }: Props) => {
  return (
    <>
      <DescriptionBlock title={m.modal_network_device_psk_generated_title()}>
        <p>{m.modal_network_device_psk_generated_content()}</p>
      </DescriptionBlock>
      <SizedBox height={ThemeSpacing.Xl2} />
      <CopyField
        label={m.form_label_preshared_key()}
        text={presharedKey}
        copyTooltip={m.misc_clipboard_copy()}
      />
      <ModalControls
        submitProps={{
          testId: 'finish',
          text: m.controls_finish(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};
