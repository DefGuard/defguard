import { zodResolver } from '@hookform/resolvers/zod';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useAddStandaloneDeviceModal } from '../../store';
import { AddStandaloneDeviceModalStep } from '../../types';

type FormFields = {
  name: string;
  location: number;
  description: string;
  assignedAddress: string;
};

export const SetupCliStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.setup;
  const labels = localLL.form.labels;
  const locationOptions = useAddStandaloneDeviceModal((s) => s.networkOptions);
  const [setState, close, next] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.close, s.changeStep],
    shallow,
  );

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .min(1, LL.form.error.required())
          .min(2, LL.form.error.minimumLength()),
        location: z.number({
          required_error: LL.form.error.required(),
          invalid_type_error: LL.form.error.invalid(),
        }),
        assignedAddress: z.string().min(1, LL.form.error.required()),
        publicKey: z.string().optional(),
      }),
    [LL.form.error],
  );

  const renderLocationOption = useCallback(
    (value: number): SelectSelectedValue => {
      if (locationOptions) {
        const option = locationOptions.find((o) => o.value === value);
        if (option) {
          return {
            displayValue: option.label,
            key: option.key,
          };
        }
        return {
          displayValue: '',
          key: 'unknown',
        };
      }
      return {
        displayValue: '',
        key: 'unknown',
      };
    },
    [locationOptions],
  );

  const { control, handleSubmit, setValue, getValues } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues: {
      assignedAddress: '',
      description: '',
      location: 0,
      name: '',
    },
  });

  const validSubmit: SubmitHandler<FormFields> = (values) => {
    console.table(values);
    next(AddStandaloneDeviceModalStep.FINISH_CLI);
  };

  useEffect(() => {
    if (locationOptions && locationOptions.length) {
      const firstId = locationOptions[0].value;
      if (getValues().location !== firstId) {
        setValue('location', firstId);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locationOptions]);

  return (
    <div className="setup-cli-step">
      <MessageBox
        type={MessageBoxType.INFO}
        message={
          // eslint-disable-next-line max-len
          'Here you can add definitions or generate configurations for devices that can connect to your VPN. Only locations without Multi-Factor Authentication are available here, as MFA is only supported in Defguard Desktop Client for now.'
        }
        dismissId="add-standalone-device-cli-setup-step-header"
      />
      <form onSubmit={handleSubmit(validSubmit)}>
        <FormInput controller={{ control, name: 'name' }} label={labels.deviceName()} />
        <div className="row">
          <FormSelect
            controller={{ control, name: 'location' }}
            options={locationOptions as NonNullable<SelectOption<number>[]>}
            renderSelected={renderLocationOption}
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
              next(AddStandaloneDeviceModalStep.FINISH_CLI);
            }}
            type="submit"
          />
        </div>
      </form>
    </div>
  );
};
