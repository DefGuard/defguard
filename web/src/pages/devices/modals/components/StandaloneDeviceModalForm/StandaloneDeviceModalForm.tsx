import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { Subject } from 'rxjs';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormLocationIp } from '../../../../../shared/defguard-ui/components/Form/FormLocationIp/FormLocationIp';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { FormToggle } from '../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ToggleOption } from '../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { GetAvailableLocationIpResponse } from '../../../../../shared/types';
import { validateWireguardPublicKey } from '../../../../../shared/validators';
import {
  AddStandaloneDeviceFormFields,
  WGConfigGenChoice,
} from '../../AddStandaloneDeviceModal/types';
import { StandaloneDeviceModalFormMode } from '../types';

type Props = {
  onSubmit: (formValues: AddStandaloneDeviceFormFields) => Promise<void>;
  mode: StandaloneDeviceModalFormMode;
  onLoadingChange: (value: boolean) => void;
  locationOptions: SelectOption<number>[];
  submitSubject: Subject<void>;
  defaults: AddStandaloneDeviceFormFields;
  reservedNames: string[];
  initialIpRecommendation: GetAvailableLocationIpResponse;
};

export const StandaloneDeviceModalForm = ({
  onSubmit,
  mode,
  onLoadingChange,
  locationOptions,
  submitSubject,
  defaults,
  reservedNames,
  initialIpRecommendation,
}: Props) => {
  const [internalRecommendedIp, setInternalRecommendedIp] = useState<
    GetAvailableLocationIpResponse | undefined
  >();
  const { LL } = useI18nContext();
  const {
    standaloneDevice: { validateLocationIp, getAvailableIp },
  } = useApi();
  // auto assign upon location change is happening
  const [ipIsLoading, setIpIsLoading] = useState(false);
  const localLL = LL.modals.addStandaloneDevice.form;
  const errors = LL.form.error;
  const labels = localLL.labels;
  const submitRef = useRef<HTMLInputElement | null>(null);
  const toaster = useToaster();
  const renderSelectedOption = useCallback(
    (val?: number): SelectSelectedValue => {
      const empty: SelectSelectedValue = {
        displayValue: '',
        key: 'empty',
      };
      if (val !== undefined) {
        const option = locationOptions.find((n) => n.value === val);
        if (option) {
          return {
            displayValue: option.label,
            key: option.key,
          };
        }
      }
      return empty;
    },
    [locationOptions],
  );

  const toggleOptions = useMemo(
    (): ToggleOption<WGConfigGenChoice>[] => [
      {
        text: labels.generation.auto(),
        value: WGConfigGenChoice.AUTO,
        disabled: false,
      },
      {
        text: labels.generation.manual(),
        value: WGConfigGenChoice.MANUAL,
        disabled: false,
      },
    ],
    [labels.generation],
  );

  const schema = useMemo(
    () =>
      z
        .object({
          name: z
            .string()
            .min(1, LL.form.error.required())
            .refine((value) => {
              if (mode === StandaloneDeviceModalFormMode.EDIT) {
                const filtered = reservedNames.filter((n) => n !== defaults.name.trim());
                return !filtered.includes(value.trim());
              }
              return !reservedNames.includes(value.trim());
            }, LL.form.error.reservedName()),
          location_id: z.number(),
          description: z.string(),
          modifiableIpPart: z.string().min(1, LL.form.error.required()),
          generationChoice: z.nativeEnum(WGConfigGenChoice),
          wireguard_pubkey: z.string().optional(),
        })
        .superRefine((vals, ctx) => {
          if (mode === StandaloneDeviceModalFormMode.CREATE_MANUAL) {
            if (vals.generationChoice === WGConfigGenChoice.MANUAL) {
              const result = validateWireguardPublicKey({
                requiredError: errors.required(),
                maxError: errors.maximumLengthOf({ length: 44 }),
                minError: errors.minimumLengthOf({ length: 44 }),
                validKeyError: errors.invalid(),
              }).safeParse(vals.wireguard_pubkey);
              if (!result.success) {
                result.error.errors.forEach((e) => {
                  ctx.addIssue({
                    path: ['wireguard_pubkey'],
                    message: e.message,
                    code: 'custom',
                  });
                });
              }
            }
          }
        }),
    [LL.form.error, defaults.name, errors, mode, reservedNames],
  );

  const {
    handleSubmit,
    control,
    watch,
    formState: { isSubmitting },
    setError,
    resetField,
  } = useForm<AddStandaloneDeviceFormFields>({
    defaultValues: defaults,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const generationChoiceValue = watch('generationChoice');

  const submitHandler: SubmitHandler<AddStandaloneDeviceFormFields> = async (
    formValues,
  ) => {
    const values = formValues;
    const { modifiableIpPart } = values;
    values.description = values.description?.trim();
    values.name = values.name.trim();
    const currentIpResp = internalRecommendedIp ?? initialIpRecommendation;
    values.modifiableIpPart =
      currentIpResp.network_part + formValues.modifiableIpPart.trim();
    if (
      mode === StandaloneDeviceModalFormMode.EDIT &&
      modifiableIpPart === defaults.modifiableIpPart
    ) {
      await onSubmit(values);
      return;
    }
    try {
      const response = await validateLocationIp({
        ip: values.modifiableIpPart,
        location: values.location_id,
      });
      const { available, valid } = response;
      if (available && valid) {
        await onSubmit(values);
      } else {
        if (!available) {
          setError('modifiableIpPart', {
            message: LL.form.error.reservedIp(),
          });
        }
        if (!valid) {
          setError('modifiableIpPart', {
            message: LL.form.error.invalidIp(),
          });
        }
      }
    } catch (e) {
      toaster.error(LL.messages.error());
      console.error(e);
    }
  };

  const autoAssignRecommendedIp = useCallback(
    (locationId: number | undefined) => {
      if (locationId !== undefined && mode !== StandaloneDeviceModalFormMode.EDIT) {
        setIpIsLoading(true);
        void getAvailableIp({
          locationId,
        })
          .then((resp) => {
            setInternalRecommendedIp(resp);
            resetField('modifiableIpPart', {
              defaultValue: resp.modifiable_part,
            });
          })
          .finally(() => {
            setIpIsLoading(false);
          });
      }
    },
    [getAvailableIp, resetField, mode],
  );

  // inform parent that form is processing stuff
  useEffect(() => {
    const res = isSubmitting;
    onLoadingChange(res);
  }, [isSubmitting, onLoadingChange]);

  // handle form sub from outside
  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      if (submitRef.current) {
        submitRef.current.click();
      }
    });
    return () => sub.unsubscribe();
  }, [submitSubject]);

  return (
    <form onSubmit={handleSubmit(submitHandler)} className="standalone-device-modal-form">
      <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
      <div className="row">
        <FormSelect
          controller={{ control, name: 'location_id' }}
          options={locationOptions}
          renderSelected={renderSelectedOption}
          label={labels.location()}
          onChangeSingle={autoAssignRecommendedIp}
          disabled={mode === StandaloneDeviceModalFormMode.EDIT}
          disableOpen={mode === StandaloneDeviceModalFormMode.EDIT}
        />
        <FormLocationIp
          controller={{ control, name: 'modifiableIpPart' }}
          data={{
            networkPart:
              internalRecommendedIp?.network_part ?? initialIpRecommendation.network_part,
            networkPrefix:
              internalRecommendedIp?.network_prefix ??
              initialIpRecommendation.network_prefix,
          }}
          label={labels.assignedAddress()}
          disabled={ipIsLoading}
        />
      </div>
      <FormInput
        controller={{ control, name: 'description' }}
        label={labels.description()}
      />
      {mode === StandaloneDeviceModalFormMode.CREATE_MANUAL && (
        <>
          <FormToggle
            controller={{ control, name: 'generationChoice' }}
            options={toggleOptions}
          />
          <FormInput
            controller={{ control, name: 'wireguard_pubkey' }}
            label={labels.publicKey()}
            disabled={generationChoiceValue === WGConfigGenChoice.AUTO}
          />
        </>
      )}
      <input className="hidden" ref={submitRef} type="submit" />
    </form>
  );
};
