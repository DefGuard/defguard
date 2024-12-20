import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { Subject } from 'rxjs';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { FormToggle } from '../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ToggleOption } from '../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
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
};

export const StandaloneDeviceModalForm = ({
  onSubmit,
  mode,
  onLoadingChange,
  locationOptions,
  submitSubject,
  defaults,
}: Props) => {
  const { LL } = useI18nContext();
  const {
    standaloneDevice: { validateLocationIp, getAvailableIp },
  } = useApi();
  // auto assign upon location change is happening
  const [ipIsLoading, setIpIsLoading] = useState(false);
  const localLL = LL.modals.addStandaloneDevice.steps.manual.setup;
  const errors = LL.form.error;
  const labels = localLL.form.labels;
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
          name: z.string().min(1, LL.form.error.required()),
          location_id: z.number(),
          description: z.string(),
          assigned_ip: z.string().min(1, LL.form.error.required()),
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
    [LL.form.error, errors, mode],
  );

  const {
    handleSubmit,
    control,
    watch,
    formState: { isSubmitting },
    setError,
    setValue,
  } = useForm<AddStandaloneDeviceFormFields>({
    defaultValues: defaults,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const generationChoiceValue = watch('generationChoice');

  const submitHandler: SubmitHandler<AddStandaloneDeviceFormFields> = async (values) => {
    if (
      mode === StandaloneDeviceModalFormMode.EDIT &&
      values.assigned_ip === defaults.assigned_ip
    ) {
      await onSubmit(values);
      return;
    }
    try {
      const response = await validateLocationIp({
        ip: values.assigned_ip,
        location: values.location_id,
      });
      const { available, valid } = response;
      if (available && valid) {
        await onSubmit(values);
      } else {
        if (!available) {
          setError('assigned_ip', {
            message: LL.form.error.reservedIp(),
          });
        }
        if (!valid) {
          setError('assigned_ip', {
            message: LL.form.error.invalidIp(),
          });
        }
      }
    } catch (e) {
      toaster.error(LL.messages.error());
    }
  };

  const autoAssignRecommendedIp = useCallback(
    (locationId: number | undefined) => {
      if (locationId !== undefined) {
        setIpIsLoading(true);
        getAvailableIp({
          locationId,
        })
          .then((resp) => setValue('assigned_ip', resp.ip))
          .finally(() => {
            setIpIsLoading(false);
          });
      }
    },
    [getAvailableIp, setValue],
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [submitSubject]);

  return (
    <form onSubmit={handleSubmit(submitHandler)} className="standalone-device-modal-form">
      <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
      <div className="row">
        <FormSelect
          controller={{ control, name: 'location_id' }}
          options={locationOptions as NonNullable<SelectOption<number>[]>}
          renderSelected={renderSelectedOption}
          label={labels.location()}
          onChangeSingle={autoAssignRecommendedIp}
          disabled={mode === StandaloneDeviceModalFormMode.EDIT}
          disableOpen={mode === StandaloneDeviceModalFormMode.EDIT}
        />
        <FormInput
          controller={{ control, name: 'assigned_ip' }}
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
