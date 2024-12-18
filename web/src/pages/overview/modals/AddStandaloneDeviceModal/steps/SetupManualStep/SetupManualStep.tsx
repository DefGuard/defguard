import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { FormToggle } from '../../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ToggleOption } from '../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { validateWireguardPublicKey } from '../../../../../../shared/validators';
import { useAddStandaloneDeviceModal } from '../../store';
import { AddStandaloneDeviceModalStep, WGConfigGenChoice } from '../../types';

type FormFields = {
  name: string;
  location: number;
  description: string;
  assignedAddress: string;
  generationChoice: WGConfigGenChoice;
  publicKey?: string;
};

export const SetupManualStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.setup;
  const errors = LL.form.error;
  const labels = localLL.form.labels;
  const locationOptions = useAddStandaloneDeviceModal((s) => s.networkOptions);
  const [setState, next] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.changeStep],
    shallow,
  );
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
          location: z.number(),
          description: z.string(),
          assignedAddress: z.string(),
          generationChoice: z.nativeEnum(WGConfigGenChoice),
          publicKey: z.string().optional(),
        })
        .superRefine((vals, ctx) => {
          if (vals.generationChoice === WGConfigGenChoice.MANUAL) {
            const result = validateWireguardPublicKey({
              requiredError: errors.required(),
              maxError: errors.maximumLengthOf({ length: 44 }),
              minError: errors.minimumLengthOf({ length: 44 }),
              validKeyError: errors.invalid(),
            }).safeParse(vals.publicKey);
            if (!result.success) {
              ctx.addIssue({
                path: ['publicKey'],
                message: result.error.message,
                code: 'custom',
              });
            }
          }
        }),
    [errors],
  );

  const { handleSubmit, control, watch } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues: {
      assignedAddress: '',
      description: '',
      generationChoice: WGConfigGenChoice.AUTO,
      name: '',
      publicKey: '',
      location: locationOptions[0].value,
    },
  });

  const generationChoiceValue = watch('generationChoice');

  const validSubmit: SubmitHandler<FormFields> = (values) => {
    console.table(values);
    setState({ genChoice: values.generationChoice });
    next(AddStandaloneDeviceModalStep.FINISH_MANUAL);
  };

  return (
    <div className="setup-manual">
      <form onSubmit={handleSubmit(validSubmit)}>
        <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
        <div className="row">
          <FormSelect
            controller={{ control, name: 'location' }}
            options={locationOptions as NonNullable<SelectOption<number>[]>}
            renderSelected={renderSelectedOption}
            label={labels.location()}
          />
          <FormInput
            controller={{ control, name: 'assignedAddress' }}
            label={labels.assignedAddress()}
          />
        </div>
        <FormInput
          controller={{ control, name: 'description' }}
          label={labels.description()}
        />
        <FormToggle
          controller={{ control, name: 'generationChoice' }}
          options={toggleOptions}
        />
        <FormInput
          controller={{ control, name: 'publicKey' }}
          label={labels.description()}
          disabled={generationChoiceValue === WGConfigGenChoice.AUTO}
        />
        <div className="controls">
          <Button
            styleVariant={ButtonStyleVariant.STANDARD}
            text={LL.common.controls.cancel()}
            onClick={() => close()}
            size={ButtonSize.LARGE}
            type="button"
          />
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={localLL.form.submit()}
            onClick={() => {
              next(AddStandaloneDeviceModalStep.FINISH_MANUAL);
            }}
            type="submit"
          />
        </div>
      </form>
    </div>
  );
};
