import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
// import {
//   type FieldValues,
//   type UseControllerProps,
//   useController,
// } from 'react-hook-form';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { RadioButton } from '../../../../../../shared/defguard-ui/components/Layout/RadioButton/Radiobutton';
// import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import type { SelectOption } from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ClientTrafficPolicy } from '../../../../../../shared/types';

type Props = {
  // controller: UseControllerProps<T>;
  // disabled?: boolean;
  onChange: (event: ClientTrafficPolicy) => void;
  fieldValue: ClientTrafficPolicy;
};

export const ClientTrafficPolicySelect = ({
  // controller,
  // disabled = false,
  onChange,
  fieldValue,
}: Props) => {
  const { LL } = useI18nContext();
  // const {
  //   field: { onChange, value: fieldValue },
  // } = useController(controller);

  const options = useMemo(
    (): SelectOption<ClientTrafficPolicy>[] => [
      {
        key: ClientTrafficPolicy.NONE,
        value: ClientTrafficPolicy.NONE,
        label: LL.components.serviceLocationModeSelect.options.disabled(),
      },
      {
        key: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        label: LL.components.serviceLocationModeSelect.options.prelogon(),
      },
      {
        key: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        label: LL.components.serviceLocationModeSelect.options.alwayson(),
      },
    ],
    [
      LL.components.serviceLocationModeSelect.options.disabled,
      LL.components.serviceLocationModeSelect.options.prelogon,
      LL.components.serviceLocationModeSelect.options.alwayson,
    ],
  );

  return (
    <div className="client-traffic-policy-select">
      <label>{LL.networkConfiguration.form.fields.service_location_mode.label()}</label>
      {options.map(({ key, value, label, disabled = false }) => {
        const active = fieldValue === value;
        return (
          <div
            className={clsx(`client-traffic-policy`, {
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
