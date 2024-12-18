import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

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
import { validateWireguardPublicKey } from '../../../../../shared/validators';
import { useAddStandaloneDeviceModal } from '../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalChoice,
  WGConfigGenChoice,
} from '../types';
import { useToaster } from '../../../../../shared/hooks/useToaster';

type FormFields = AddStandaloneDeviceFormFields;

type Props = {
  onSubmit: (formValues: FormFields) => Promise<void>;
  defaultValues: FormFields;
  mode: AddStandaloneDeviceModalChoice;
  initialAssignedIp: string;
  onLoadingChange: (value: boolean) => void;
};

export const StandaloneDeviceModalForm = ({
  onSubmit,
  mode,
  onLoadingChange,
  initialAssignedIp,
}: Props) => {
  const { LL } = useI18nContext();
  const {
    standaloneDevice: { validateLocationIp, getAvailableIp },
  } = useApi();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.setup;
  const errors = LL.form.error;
  const labels = localLL.form.labels;
  const locationOptions = useAddStandaloneDeviceModal((s) => s.networkOptions);
  const submitRef = useRef<HTMLInputElement | null>(null);
  const submitSubject = useAddStandaloneDeviceModal((s) => s.submitSubject, shallow);
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
          name: z.string(),
          location_id: z.number(),
          description: z.string(),
          assigned_ip: z.string(),
          generationChoice: z.nativeEnum(WGConfigGenChoice),
          wireguard_pubkey: z.string().optional(),
        })
        .superRefine((vals, ctx) => {
          if (mode === AddStandaloneDeviceModalChoice.MANUAL) {
            if (vals.generationChoice === WGConfigGenChoice.MANUAL) {
              const result = validateWireguardPublicKey({
                requiredError: errors.required(),
                maxError: errors.maximumLengthOf({ length: 44 }),
                minError: errors.minimumLengthOf({ length: 44 }),
                validKeyError: errors.invalid(),
              }).safeParse(vals.wireguard_pubkey);
              if (!result.success) {
                ctx.addIssue({
                  path: ['wireguard_pubkey'],
                  message: result.error.message,
                  code: 'custom',
                });
              }
            }
          }
        }),
    [errors, mode],
  );

  const {
    handleSubmit,
    control,
    watch,
    formState: { isSubmitting },
    setError,
  } = useForm<AddStandaloneDeviceFormFields>({
    defaultValues: {
      description: '',
      generationChoice: WGConfigGenChoice.AUTO,
      name: '',
      location_id: locationOptions[0].value,
      assigned_ip: initialAssignedIp,
      wireguard_pubkey: '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const generationChoiceValue = watch('generationChoice');

  const locationId = watch('location_id');

  const submitHandler: SubmitHandler<AddStandaloneDeviceFormFields> = async (values) => {
    // TODO: validate ip here
    // await actual handler after post submit validation is done
    try {
      // TODO: Needs API changes
      const isValid = await validateLocationIp({
        ip: values.assigned_ip,
        location: values.location_id,
      });
      if (isValid) {
        await onSubmit(values);
      } else {
        // FIXME: change the label
        setError('assigned_ip', {
          message: 'Invalid IP (change the label)',
        });
      }
    } catch (e) {
      toaster.error('Something went wrong.', 'Please try again.');
    }
  };

  // reassign ip on change
  useEffect(() => {}, [locationId]);

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
  }, []);

  return (
    <form onSubmit={handleSubmit(submitHandler)}>
      <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
      <div className="row">
        <FormSelect
          controller={{ control, name: 'location_id' }}
          options={locationOptions as NonNullable<SelectOption<number>[]>}
          renderSelected={renderSelectedOption}
          label={labels.location()}
        />
        <FormInput
          controller={{ control, name: 'assigned_ip' }}
          label={labels.assignedAddress()}
        />
      </div>
      <FormInput
        controller={{ control, name: 'description' }}
        label={labels.description()}
      />
      {mode === AddStandaloneDeviceModalChoice.MANUAL && (
        <>
          <FormToggle
            controller={{ control, name: 'generationChoice' }}
            options={toggleOptions}
          />
          <FormInput
            controller={{ control, name: 'wireguard_pubkey' }}
            label={labels.description()}
            disabled={generationChoiceValue === WGConfigGenChoice.AUTO}
          />
        </>
      )}
      <input className="hidden" ref={submitRef} type="submit" />
    </form>
  );
};
