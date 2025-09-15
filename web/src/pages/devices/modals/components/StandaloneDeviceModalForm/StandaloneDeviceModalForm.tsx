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

  const submitHandler: SubmitHandler<StandaloneDeviceFormFields> = async (formValues) => {
    const values = formValues;
    const recommendationResponse = internalRecommendedIps ?? initialIpRecommendation;
    let validationList = recommendationResponse.map((recommendation, index) => ({
      ip: recommendation.network_part + formValues.modifiableIpParts[index],
      index,
    }));
    values.modifiableIpParts = validationList.map((item) => item.ip);
    // try to validate explicitly chosen IPs before submission
    let validationErrors = false;

    // if edit exclude initial ip's from validation as they are reserved already by edited device
    if (mode === StandaloneDeviceModalFormMode.EDIT) {
      const reservedByDevice = initialIpRecommendation.map(
        (item) => item.network_part + item.modifiable_part,
      );
      validationList = validationList.filter(
        (item) => !reservedByDevice.includes(item.ip),
      );
    }

    if (validationList.length) {
      try {
        const response = await validateLocationIp({
          ips: validationList.map((item) => item.ip),
          location: values.location_id,
        });

        response.forEach(({ available, valid }, index) => {
          const fieldIndex = validationList[index].index;
          if (!available) {
            validationErrors = true;
            setError(`modifiableIpParts.${fieldIndex}`, {
              message: LL.form.error.reservedIp(),
            });
          }
          if (!valid) {
            validationErrors = true;
            setError(`modifiableIpParts.${fieldIndex}`, {
              message: LL.form.error.invalidIp(),
            });
          }
        });
      } catch (_) {
        validationErrors = true;
        toaster.error(LL.messages.error());
      }
    }

    // submit form if no validation errors occurred
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
