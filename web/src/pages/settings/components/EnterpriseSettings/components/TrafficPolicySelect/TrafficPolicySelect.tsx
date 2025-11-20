import './style.scss';
import clsx from 'clsx';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { RadioButton } from '../../../../../../shared/defguard-ui/components/Layout/RadioButton/Radiobutton';
import type { SelectOption } from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ClientTrafficPolicy } from '../../../../../../shared/types';

type Props = {
  onChange: (event: ClientTrafficPolicy) => void;
  fieldValue: ClientTrafficPolicy;
};

export const ClientTrafficPolicySelect = ({ onChange, fieldValue }: Props) => {
  const { LL } = useI18nContext();
  const options = useMemo(
    (): SelectOption<ClientTrafficPolicy>[] => [
      {
        key: ClientTrafficPolicy.NONE,
        value: ClientTrafficPolicy.NONE,
        label: LL.settingsPage.enterprise.fields.clientTrafficPolicy.none.label(),
        meta: LL.settingsPage.enterprise.fields.clientTrafficPolicy.none.helper(),
      },
      {
        key: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        label:
          LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.label(),
        meta: LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.helper(),
      },
      {
        key: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        label:
          LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.label(),
        meta: LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.helper(),
      },
    ],
    [
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.none.label,
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.none.helper,
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.label,
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.helper,
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.label,
      LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.helper,
    ],
  );

  return (
    <div className="client-traffic-policy-select">
      <label>{LL.settingsPage.enterprise.fields.clientTrafficPolicy.header()}</label>
      {options.map(({ key, value, label, meta, disabled = false }) => {
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
            <Helper>{parse(meta)}</Helper>
          </div>
        );
      })}
    </div>
  );
};
