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
import { useAppStore } from '../../../hooks/store/useAppStore';
import { ServiceLocationMode } from '../../../types';

type Props<T extends FieldValues> = {
  controller: UseControllerProps<T>;
  disabled?: boolean;
};

export const FormServiceLocationModeSelect = <T extends FieldValues>({
  controller,
  disabled = false,
}: Props<T>) => {
  const { LL } = useI18nContext();
  const {
    field: { onChange, value: fieldValue },
  } = useController(controller);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  const options = useMemo(
    (): SelectOption<ServiceLocationMode>[] => [
      {
        key: ServiceLocationMode.DISABLED,
        value: ServiceLocationMode.DISABLED,
        label: LL.components.serviceLocationModeSelect.options.disabled(),
      },
      {
        key: ServiceLocationMode.PRELOGON,
        value: ServiceLocationMode.PRELOGON,
        label: LL.components.serviceLocationModeSelect.options.prelogon(),
        disabled: !enterpriseEnabled || disabled,
      },
      {
        key: ServiceLocationMode.ALWAYSON,
        value: ServiceLocationMode.ALWAYSON,
        label: LL.components.serviceLocationModeSelect.options.alwayson(),
        disabled: !enterpriseEnabled || disabled,
      },
    ],
    [
      LL.components.serviceLocationModeSelect.options.disabled,
      LL.components.serviceLocationModeSelect.options.prelogon,
      LL.components.serviceLocationModeSelect.options.alwayson,
      disabled,
      enterpriseEnabled,
    ],
  );

  return (
    <div className="service-location-mode-select">
      <label>{LL.networkConfiguration.form.fields.service_location_mode.label()}</label>
      {options.map(({ key, value, label, disabled = false }) => {
        const active = fieldValue === value;
        return (
          <div
            className={clsx(`service-location-mode`, {
              active,
              disabled,
            })}
            key={key}
            onClick={() => {
              if (!disabled) {
                onChange(value);
              }
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
