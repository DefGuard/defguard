import './style.scss';

import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguardNavLogo from '../../../../shared/components/svg/DefguardNavLogo';
import SvgWireguardLogo from '../../../../shared/components/svg/WireguardLogo';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import { AddDeviceMethod } from '../../types';
import { DeviceSetupMethodCard } from './components/DeviceSetupMethodCard/DeviceSetupMethodCard';

export const AddDeviceSetupMethodStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.setupMethod;
  const setupMethod = useAddDevicePageStore((state) => state.method);
  const [setPageState, next, nextSubject] = useAddDevicePageStore(
    (state) => [state.setState, state.nextStep, state.nextSubject],
    shallow,
  );

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      next();
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, next]);

  return (
    <>
      <MessageBox
        type={MessageBoxType.WARNING}
        message={LL.addDevicePage.helpers.setupOpt()}
        dismissId="add-device-page-method-opt-message"
      />
      <Card shaded id="setup-method-step">
        <DeviceSetupMethodCard
          title={localLL.remote.title()}
          subtitle={localLL.remote.subTitle()}
          logo={<SvgDefguardNavLogo />}
          linkText={localLL.remote.link()}
          link={`https://github.com/DefGuard/client/releases`}
          selected={setupMethod === AddDeviceMethod.DESKTOP}
          onSelect={() => {
            if (setupMethod !== AddDeviceMethod.DESKTOP) {
              setPageState({ method: AddDeviceMethod.DESKTOP });
            }
          }}
        />
        <DeviceSetupMethodCard
          title={localLL.manual.title()}
          subtitle={localLL.manual.subTitle()}
          logo={<SvgWireguardLogo />}
          linkText={localLL.manual.link()}
          link={`https://www.wireguard.com/install/`}
          selected={setupMethod === AddDeviceMethod.MANUAL}
          onSelect={() => {
            if (setupMethod !== AddDeviceMethod.MANUAL) {
              setPageState({ method: AddDeviceMethod.MANUAL });
            }
          }}
        />
      </Card>
    </>
  );
};
