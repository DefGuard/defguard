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
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const externalOpenIdConfigured = useAppStore((s) => s.appInfo?.external_openid_enabled);
  const externalMfaDisabled = !(enterpriseEnabled && externalOpenIdConfigured);

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
        disabled: externalMfaDisabled,
      },
    ],
    [
      LL.components.locationMfaModeSelect.options.disabled,
      LL.components.locationMfaModeSelect.options.external,
      LL.components.locationMfaModeSelect.options.internal,
      externalMfaDisabled,
    ],
  );

  return (
    <div className="location-mfa-mode-select">
      <label>{LL.networkConfiguration.form.fields.location_mfa_mode.label()}</label>
      {options.map(({ key, value, label, disabled = false }) => {
        const active = fieldValue === value;
        return (
          <div
            className={clsx(`location-mfa-mode`, {
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
