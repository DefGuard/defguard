import { yupResolver } from '@hookform/resolvers/yup';
import { isUndefined } from 'lodash-es';
import React from 'react';
import { useEffect } from 'react';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import { useFieldArray } from 'react-hook-form';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { Input } from '../../../../../shared/components/layout/Input/Input';
import SvgIconPlusGray from '../../../../../shared/components/svg/IconPlusGray';
import { useWizardStore } from '../store';

type SharedField = {
  ipAddress: string;
};

type Inputs = {
  name: string;
  ipAddress: string;
  shared: SharedField[];
};

const StepLocationsForm: React.FC = () => {
  const { LL } = useI18nContext();
  const schema = yup
    .object({
      name: yup.string().required(),
      ipAddress: yup
        .string()
        .required()
        .matches(
          /^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])(:\d{1,5}\b)?$/,
          LL.wizard.locations.form.validation.invalidAddress()
        ),
      shared: yup
        .array()
        .of(
          yup.object({
            ipAddress: yup
              .string()
              .required()
              .matches(
                /^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])(:\d{1,5}\b)?$/,
                LL.wizard.locations.form.validation.invalidAddress()
              ),
          })
        )
        .required()
        .min(1),
    })
    .required();
  const { handleSubmit, control, reset } = useForm<Inputs>({
    mode: 'all',
    resolver: yupResolver(schema),
    defaultValues: {
      ipAddress: '',
      name: '',
      shared: [{ ipAddress: '' }] as SharedField[],
    },
  });
  const { fields, append, remove } = useFieldArray({
    control: control,
    name: 'shared',
  });
  const addLocation = useWizardStore((state) => state.addLocation);
  const locationsCount = useWizardStore((state) => state.locations.length);
  const onSubmit: SubmitHandler<Inputs> = (data) => {
    addLocation(data);
    reset({ name: '', ipAddress: '' });
    // NOTE: Resetting fieldArray instead of calling remove() will result in error due to fieldArray being set to incorrect type or value.
    remove();
    append({} as SharedField);
  };
  const setFormStatus = useWizardStore((state) => state.setFormStatus);

  useEffect(() => {
    if (locationsCount > 0) {
      setFormStatus({ 2: true });
    } else {
      setFormStatus({ 2: false });
    }
  }, [locationsCount, setFormStatus]);

  return (
    <div className="location-form">
      <h2>New location:</h2>
      <form onSubmit={handleSubmit(onSubmit)}>
        <div className="inputs-container">
          <FormInput
            controller={{ control, name: 'name' }}
            placeholder="Location name"
            required
          />
          <FormInput
            controller={{ control, name: 'ipAddress' }}
            placeholder="Location IP Address"
            required
          />
          <p className="inputs-field-info">
            Add at least one IP address this location will share:
          </p>
          {fields.map((field, index) => (
            <Controller
              key={field.id}
              control={control}
              defaultValue={field.ipAddress || ''}
              name={`shared.${index}.ipAddress`}
              render={({ field, fieldState }) => (
                <Input
                  value={field.value}
                  onChange={(e) => field.onChange(e.target.value)}
                  type="text"
                  placeholder="Eg. 20.2.200.20"
                  required
                  disposable={index !== 0}
                  disposeHandler={() => remove(index)}
                  invalid={
                    !isUndefined(fieldState.error) && fieldState.isTouched
                  }
                  valid={!fieldState.error && fieldState.isTouched}
                  errorMessage={fieldState.error?.message}
                />
              )}
            />
          ))}
        </div>

        <div
          className="icon-with-label add-address"
          onClick={() => append({} as SharedField)}
        >
          <button className="icon-button" type="button">
            <SvgIconPlusGray />
          </button>
          <p>Add another address</p>
        </div>
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          text="Add location"
        />
      </form>
    </div>
  );
};

export default StepLocationsForm;
