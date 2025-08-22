import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import type { Subject } from 'rxjs';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormLocationIp } from '../../../../../shared/defguard-ui/components/Form/FormLocationIp/FormLocationIp';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { FormToggle } from '../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import type {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import type { ToggleOption } from '../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import type { GetAvailableLocationIpResponse } from '../../../../../shared/types';
import {
  type AddStandaloneDeviceFormFields,
  WGConfigGenChoice,
} from '../../AddStandaloneDeviceModal/types';
import { StandaloneDeviceModalFormMode } from '../types';
import {
  type StandaloneDeviceFormFields,
  standaloneDeviceFormSchema,
} from './formSchema';

type Props = {
  onSubmit: (formValues: AddStandaloneDeviceFormFields) => Promise<void>;
  mode: StandaloneDeviceModalFormMode;
  onLoadingChange: (value: boolean) => void;
  locationOptions: SelectOption<number>[];
  submitSubject: Subject<void>;
  defaults: StandaloneDeviceFormFields;
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
  const [internalRecommendedIps, setInternalRecommendedIps] = useState<
    GetAvailableLocationIpResponse | undefined
  >();
  const { LL } = useI18nContext();
  const {
    standaloneDevice: { validateLocationIp, getAvailableIp },
  } = useApi();
  // auto assign upon location change is happening
  const [ipIsLoading, setIpIsLoading] = useState(false);
  const localLL = LL.modals.addStandaloneDevice.form;
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
      standaloneDeviceFormSchema(LL, {
        mode,
        reservedNames,
        originalName: defaults.name,
      }),
    [mode, reservedNames, defaults.name, LL],
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

  function newIps(formIps: string[]): string[] {
    const initialIpsSet = new Set<string>(
      initialIpRecommendation.map((ip) => ip.network_part + ip.modifiable_part),
    );
    const formIpsSet = new Set<string>(formIps);
    return Array.from(formIpsSet.difference(initialIpsSet));
  }
  const submitHandler: SubmitHandler<StandaloneDeviceFormFields> = async (formValues) => {
    const values = formValues;
    const { modifiableIpParts: modifiableIpPart } = values;
    values.description = values.description?.trim();
    values.name = values.name.trim();
    const currentIpResp = internalRecommendedIps ?? initialIpRecommendation;
    values.modifiableIpParts = currentIpResp.map(
      (resp, i) => resp.network_part + formValues.modifiableIpParts[i].trim(),
    );
    if (
      mode === StandaloneDeviceModalFormMode.EDIT &&
      modifiableIpPart === defaults.modifiableIpParts
    ) {
      await onSubmit(values);
      return;
    }
    const ips = newIps(values.modifiableIpParts);
    let validationErrors = false;
    let index = 0;
    for (const newIp of ips) {
      try {
        const response = await validateLocationIp({
          ips: [newIp],
          location: values.location_id,
        });
        const { available, valid } = response;
        if (!available) {
          validationErrors = true;
          setError(`modifiableIpParts.${index}`, {
            message: LL.form.error.reservedIp(),
          });
        }
        if (!valid) {
          validationErrors = true;
          setError(`modifiableIpParts.${index}`, {
            message: LL.form.error.invalidIp(),
          });
        }
      } catch (_) {
        validationErrors = true;
      } finally {
        index++;
      }
    }
    if (!validationErrors) {
      try {
        await onSubmit(values);
      } catch (_) {
        toaster.error(LL.messages.error());
      }
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
            setInternalRecommendedIps(resp);
            resetField('modifiableIpParts', {
              defaultValue: resp.map((r) => r.modifiable_part),
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

  const recommendedIps = internalRecommendedIps || initialIpRecommendation;
  return (
    <form onSubmit={handleSubmit(submitHandler)} className="standalone-device-modal-form">
      <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
      <FormSelect
        controller={{ control, name: 'location_id' }}
        options={locationOptions}
        renderSelected={renderSelectedOption}
        label={labels.location()}
        onChangeSingle={autoAssignRecommendedIp}
        disabled={mode === StandaloneDeviceModalFormMode.EDIT}
        disableOpen={mode === StandaloneDeviceModalFormMode.EDIT}
      />
      <FormInput
        controller={{ control, name: 'description' }}
        label={labels.description()}
      />
      {recommendedIps.map((ip, i) => (
        <FormLocationIp
          key={i}
          controller={{ control, name: `modifiableIpParts.${i}` }}
          data={{
            networkPart: ip?.network_part,
            networkPrefix: ip?.network_prefix,
          }}
          label={labels.assignedAddress()}
          disabled={ipIsLoading}
        />
      ))}
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
