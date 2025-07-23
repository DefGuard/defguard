import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
import {
  type FieldValues,
  type UseControllerProps,
  useController,
} from 'react-hook-form';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { RadioButton } from '../../../defguard-ui/components/Layout/RadioButton/Radiobutton';
import type { SelectOption } from '../../../defguard-ui/components/Layout/Select/types';
import { LocationMfaMode } from '../../../types';

type Props<T extends FieldValues> = {
  controller: UseControllerProps<T>;
};

export const FormLocationMfaModeSelect = <T extends FieldValues>({
  controller,
}: Props<T>) => {
  const { LL } = useI18nContext();
  const {
    field: { onChange, value: fieldValue },
  } = useController(controller);

  const options = useMemo(
    (): SelectOption<LocationMfaMode>[] => [
      {
        key: LocationMfaMode.DISABLED,
        value: LocationMfaMode.DISABLED,
        label: LL.components.locationMfaModeSelect.options.disabled(),
      },
      {
        key: LocationMfaMode.INTERNAL,
        value: LocationMfaMode.INTERNAL,
        label: LL.components.locationMfaModeSelect.options.internal(),
      },
      {
        key: LocationMfaMode.EXTERNAL,
        value: LocationMfaMode.EXTERNAL,
        label: LL.components.locationMfaModeSelect.options.external(),
      },
    ],
    [LL.components.locationMfaModeSelect.options],
  );

  return (
    <div className="location-mfa-mode-select">
      <label>{LL.networkConfiguration.form.fields.location_mfa_mode.label()}</label>
      {options.map(({ key, value, label }) => {
        const active = fieldValue === value;
        return (
          <div
            className={clsx(`location-mfa-mode ${value}`, {
              active,
            })}
            key={key}
            onClick={() => {
              onChange(value);
            }}
          >
            <p className="label">{label}</p>
            <RadioButton active={active} />
          </div>
        );
      })}
    </div>
  );
};
