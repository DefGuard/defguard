import z from 'zod';
import api from '../../../../shared/api/api';
import type {
  AddNetworkDeviceResponse,
  AvailableLocationIP,
  StartEnrollmentResponse,
} from '../../../../shared/api/types';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import type {
  SelectOption,
  SelectSingleValue,
} from '../../../../shared/defguard-ui/components/Select/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { FormSection } from '../../../../shared/components/FormSection/FormSection';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { CodeBox } from '../../../../shared/defguard-ui/components/CodeBox/CodeBox';
import { CopyField } from '../../../../shared/defguard-ui/components/CopyField/CopyField';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { Select } from '../../../../shared/defguard-ui/components/Select/Select';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useClipboard } from '../../../../shared/defguard-ui/hooks/useClipboard';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { patternValidWireguardKey } from '../../../../shared/patterns';
import { downloadText } from '../../../../shared/utils/download';
import { formatFileName } from '../../../../shared/utils/formatFileName';
import { generateWGKeys } from '../../../../shared/utils/generateWGKeys';

const modalNameValue = ModalName.AddNetworkDevice;

type ModalStep = 'choice' | 'form' | 'cli-finish' | 'show-config';

type LocationOptions = SelectOption<number>[];

interface ModalState {
  step: ModalStep;
  useCli: boolean;
  reservedNames: string[];
  initialAvailableIps: AvailableLocationIP[];
  locationOptions: LocationOptions;
  manualDevice?: AddNetworkDeviceResponse;
  cliDevice?: StartEnrollmentResponse;
  privateKey?: string;
}

interface StepProps {
  setModalState: (values: Partial<ModalState>) => void;
}

const defaultModalState: ModalState = {
  step: 'choice',
  useCli: false,
  reservedNames: [],
  initialAvailableIps: [],
  locationOptions: [],
  privateKey: undefined,
  cliDevice: undefined,
  manualDevice: undefined,
};

export const AddNetworkDeviceModal = () => {
  const [modalState, setModalState] = useState<ModalState>(defaultModalState);
  const [isOpen, setOpen] = useState(false);

  const handleStepStateChange = useCallback((value: Partial<ModalState>) => {
    setModalState((s) => ({ ...s, ...value }));
  }, []);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setOpen(true);
      setModalState({
        ...defaultModalState,
        reservedNames: data.reservedNames,
        initialAvailableIps: data.availableIps,
        locationOptions: data.locations.map((location) => ({
          key: location.id,
          label: location.name,
          value: location.id,
        })),
      });
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="add-network-device-modal"
      title={'Add new network device'}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalState(defaultModalState);
      }}
    >
      {isPresent(modalState) && (
        <>
          {modalState.step === 'choice' && (
            <ChoiceStep setModalState={handleStepStateChange} />
          )}
          {modalState.step === 'form' && (
            <FormStep setModalState={handleStepStateChange} {...modalState} />
          )}
          {modalState.step === 'cli-finish' && isPresent(modalState.cliDevice) && (
            <CliStep data={modalState.cliDevice} />
          )}
          {modalState.step === 'show-config' && (
            <ManualStep
              manualDevice={modalState.manualDevice}
              privateKey={modalState.privateKey}
            />
          )}
        </>
      )}
    </Modal>
  );
};

const ManualStep = ({
  manualDevice,
  privateKey,
}: Pick<ModalState, 'manualDevice' | 'privateKey'>) => {
  const { writeToClipboard } = useClipboard();
  const config = useMemo(() => {
    if (!isPresent(manualDevice) || !isPresent(privateKey)) return null;
    let res = manualDevice.config.config;
    if (privateKey) {
      res = res.replace('YOUR_PRIVATE_KEY', privateKey);
    }
    return res;
  }, [manualDevice, privateKey]);

  if (!config || !manualDevice) return null;

  return (
    <>
      <FormSection
        title="Get configuration file"
        text="Use the provided configuration file by importing it into your device's WireGuard app."
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <InfoBanner
        variant="warning"
        icon="warning"
        text={
          "Defguard doesn't store private keys. Keys are generated in your browser — only the public key is saved. Download the configuration now; the private key won't be available later."
        }
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <CodeBox text={config.replaceAll('\n', '<br/>')} markdown />
      <SizedBox height={ThemeSpacing.Xl2} />
      <div className="box-controls">
        <Button
          variant="outlined"
          text="Download config file"
          iconLeft="download"
          onClick={() => {
            downloadText(config, formatFileName(manualDevice.device.name), 'conf');
          }}
        />
        <Button
          variant="outlined"
          text={m.controls_copy_clipboard()}
          iconLeft="copy"
          onClick={() => {
            writeToClipboard(config);
          }}
        />
      </div>
      <ModalControls
        submitProps={{
          text: m.controls_finish(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};

const CliStep = ({ data }: { data: StartEnrollmentResponse }) => {
  const command = `dg enroll -u ${data.enrollment_url} -t ${data.enrollment_token}`;
  return (
    <>
      <AppText font={TextStyle.TBodySm500}>Activate your device in terminal</AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        Copy and paste the command below to authenticate and configure your Defguard
        Command Line Client.
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      <CopyField label="Command" text={command} copyTooltip={m.misc_clipboard_copy()} />
      <ModalControls
        submitProps={{
          text: m.controls_finish(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};

const ChoiceStep = ({ setModalState }: StepProps) => {
  const handleSelect = useCallback(
    (useCli: boolean) => {
      setModalState({
        step: 'form',
        useCli,
      });
    },
    [setModalState],
  );

  return (
    <>
      <SectionSelect
        image="device-clc"
        title="Defguard Command Line Client"
        content="When using Defguard CLI your device will be automatically configured."
        onClick={() => {
          handleSelect(true);
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="wireguard-device"
        title="Manual WireGuard Client"
        content="If your device doesn't support our CLI, you can generate a WireGuard config and set it up manually — but future location updates must be applied manually."
        onClick={() => {
          handleSelect(false);
        }}
      />
    </>
  );
};

type SubmitError = {
  [K: `modifiableIpParts[${number}]`]: string;
};

const mutationMeta = {
  invalidate: ['device', 'network'],
};

const FormStep = ({
  useCli,
  initialAvailableIps,
  locationOptions,
  reservedNames,
  setModalState,
}: StepProps & ModalState) => {
  const { mutateAsync: addDevice } = useMutation({
    mutationFn: api.network_device.addDevice,
    meta: mutationMeta,
  });
  const { mutateAsync: startCli } = useMutation({
    mutationFn: api.network_device.addCliDevice,
    meta: mutationMeta,
  });
  const [selected, setSelected] = useState<SelectSingleValue<number>>(locationOptions[0]);
  const [availableIps, setAvailableIps] =
    useState<AvailableLocationIP[]>(initialAvailableIps);

  const formSchema = useMemo(
    () =>
      z
        .object({
          name: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required())
            .refine(
              (value) => !reservedNames.includes(value),
              m.form_error_name_reserved(),
            ),
          description: z.string().trim().nullable(),
          modifiableIpParts: z.array(
            z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
          ),
          generateKeys: z.boolean(),
          wireguard_pubkey: z.string().trim().nullable(),
        })
        .superRefine((values, ctx) => {
          if (!values.generateKeys) {
            const result = z
              .string(m.form_error_required())
              .regex(patternValidWireguardKey, m.form_error_invalid_key())
              .safeParse(values.wireguard_pubkey);
            if (!result.success) {
              ctx.addIssue({
                code: 'custom',
                message: result.error.issues[0].message,
                path: ['wireguard_pubkey'],
              });
            }
          }
        }),
    [reservedNames.includes],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      name: '',
      generateKeys: true,
      modifiableIpParts: initialAvailableIps.map((item) => item.modifiable_part),
      description: null,
      wireguard_pubkey: null,
    }),
    [initialAvailableIps],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      const formIpList = availableIps.map(
        (item, index) => item.network_part + value.modifiableIpParts[index],
      );
      const { data: validationResponse } = await api.network_device.validateIps({
        ips: formIpList,
        locationId: selected.value,
      });
      const errors: SubmitError = {};
      validationResponse.forEach(({ available, valid }, index) => {
        if (!valid) {
          errors[`modifiableIpParts[${index}]`] = m.form_error_ip_invalid();
        }
        if (!available) {
          errors[`modifiableIpParts[${index}]`] = m.form_error_ip_reserved();
        }
      });
      if (Object.keys(errors).length) {
        formApi.setErrorMap({
          onSubmit: {
            fields: errors,
          },
        });
        return;
      }
      let privateKey: string | undefined;
      let publicKey: string | null = value.wireguard_pubkey;
      if (value.generateKeys && !useCli) {
        const keys = generateWGKeys();
        privateKey = keys.privateKey;
        publicKey = keys.publicKey;
      }

      if (useCli) {
        const { data: enrollment } = await startCli({
          assigned_ips: formIpList,
          location_id: selected.value,
          name: value.name,
          description: value.description,
          wireguard_pubkey: publicKey,
        });
        setModalState({
          step: 'cli-finish',
          cliDevice: enrollment,
        });
      } else {
        const { data: createdDevice } = await addDevice({
          assigned_ips: formIpList,
          location_id: selected.value,
          name: value.name,
          description: value.description,
          wireguard_pubkey: publicKey,
        });
        setModalState({
          step: 'show-config',
          manualDevice: createdDevice,
          privateKey,
        });
      }
    },
  });

  const genValue = useStore(form.store, (store) => store.values.generateKeys);

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  const handleLocationChange = useCallback(
    async (option: SelectSingleValue<number>) => {
      setSelected(option);
      const { data: newIpSuggestions } = await api.network_device.getAvailableIp(
        option.value,
      );
      setAvailableIps(newIpSuggestions);
      form.setFieldValue(
        'modifiableIpParts',
        newIpSuggestions.map((item) => item.modifiable_part),
        {
          dontValidate: true,
        },
      );
    },
    [form.setFieldValue],
  );

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <InfoBanner
          variant="warning"
          icon="info-outlined"
          text="Only locations without Multi-Factor Authentication are shown in the selector below, as MFA is currently supported only in the Defguard Desktop Client."
        />
        <SizedBox height={ThemeSpacing.Xl} />
        <div className="fields">
          <Select
            options={locationOptions}
            value={selected}
            onChange={handleLocationChange}
            label="Location"
            required
          />
          <form.AppField name="name">
            {(field) => <field.FormInput required label={'Device name'} />}
          </form.AppField>
          <form.AppField name="description">
            {(field) => <field.FormInput label={'Description'} />}
          </form.AppField>
          <form.AppField name="modifiableIpParts" mode="array">
            {(field) =>
              field.state.value.map((_, index) => (
                <form.AppField key={index} name={`modifiableIpParts[${index}]`}>
                  {(subField) => (
                    <subField.FormSuggestedIPInput
                      data={availableIps[index]}
                      required
                      label="Assigned IP Address"
                    />
                  )}
                </form.AppField>
              ))
            }
          </form.AppField>
        </div>
        {/* Gen options only for manual flow */}
        {!useCli && (
          <>
            <SizedBox height={ThemeSpacing.Xl3} />
            <AppText font={TextStyle.TBodyPrimary500}>
              {'Specify your private keys'}
            </AppText>
            <SizedBox height={ThemeSpacing.Xl} />
            <form.AppField name="generateKeys">
              {(field) => <field.FormRadio value={true} text="Generate a key pair" />}
            </form.AppField>
            <SizedBox height={ThemeSpacing.Md} />
            <form.AppField name="generateKeys">
              {(field) => (
                <field.FormRadio value={false} text="Provide your own public key" />
              )}
            </form.AppField>
            {!genValue && (
              <>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="wireguard_pubkey">
                  {(field) => <field.FormInput required label="Public key" />}
                </form.AppField>
              </>
            )}
          </>
        )}
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
        />
      </form.AppForm>
    </form>
  );
};
