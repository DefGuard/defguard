import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
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
      },
      {
        key: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
        label:
          LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.label(),
      },
      {
        key: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        value: ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
        label:
          LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.label(),
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
      <MessageBox id="client-traffic-policy-message-box">
        <ul>
          <li>
            <p>{LL.settingsPage.enterprise.fields.clientTrafficPolicy.none.helper()}</p>
          </li>
          <li>
            <p>
              {LL.settingsPage.enterprise.fields.clientTrafficPolicy.disableAllTraffic.helper()}
            </p>
          </li>
          <li>
            <p>
              {LL.settingsPage.enterprise.fields.clientTrafficPolicy.forceAllTraffic.helper()}
            </p>
          </li>
        </ul>
      </MessageBox>
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
