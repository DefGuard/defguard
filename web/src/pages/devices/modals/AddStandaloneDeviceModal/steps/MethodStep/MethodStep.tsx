import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useId } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import SvgWireguardLogo from '../../../../../../shared/components/svg/WireguardLogo';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { SelectOption } from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { externalLink } from '../../../../../../shared/links';
import { QueryKeys } from '../../../../../../shared/queries';
import { Network } from '../../../../../../shared/types';
import { DeviceSetupMethodCard } from '../../../../../addDevice/steps/AddDeviceSetupMethodStep/components/DeviceSetupMethodCard/DeviceSetupMethodCard';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceModalChoice,
  AddStandaloneDeviceModalStep,
} from '../../types';

export const MethodStep = () => {
  // this is needs bcs opening modal again and again would prevent availableIp to refetch
  const modalSessionID = useId();
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.method;
  const choice = useAddStandaloneDeviceModal((s) => s.choice);
  const {
    network: { getNetworks },
    standaloneDevice: { getAvailableIp },
  } = useApi();

  const { data: networks } = useQuery({
    queryKey: [QueryKeys.FETCH_NETWORKS],
    queryFn: getNetworks,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const {
    data: availableIpResponse,
    refetch: refetchAvailableIp,
    isLoading: availableIpLoading,
  } = useQuery({
    queryKey: [
      'ADD_STANDALONE_DEVICE_MODAL_FETCH_INITIAL_AVAILABLE_IP',
      networks,
      modalSessionID,
    ],
    queryFn: () =>
      getAvailableIp({
        locationId: (networks as Network[])[0].id,
      }),
    enabled: networks !== undefined && Array.isArray(networks) && networks.length > 0,
    refetchOnMount: true,
    refetchOnReconnect: true,
    refetchOnWindowFocus: false,
  });

  const [setState, close, next] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.close, s.changeStep],
    shallow,
  );

  const handleChange = useCallback(
    (choice: AddStandaloneDeviceModalChoice) => {
      setState({ choice });
    },
    [setState],
  );

  const handleNext = () => {
    switch (choice) {
      case AddStandaloneDeviceModalChoice.CLI:
        next(AddStandaloneDeviceModalStep.SETUP_CLI);
        break;
      case AddStandaloneDeviceModalChoice.MANUAL:
        next(AddStandaloneDeviceModalStep.SETUP_MANUAL);
        break;
    }
  };

  useEffect(() => {
    if (networks) {
      const options: SelectOption<number>[] = networks.map((n) => ({
        key: n.id,
        value: n.id,
        label: n.name,
      }));
      setState({
        networks,
        networkOptions: options,
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [networks]);

  useEffect(() => {
    if (availableIpResponse) {
      setState({ initLocationIpResponse: availableIpResponse });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [availableIpResponse]);

  return (
    <div className="method-step">
      <div className="choices">
        <DeviceSetupMethodCard
          title={localLL.cards.cli.title()}
          subtitle={localLL.cards.cli.subtitle()}
          link={externalLink.defguardCliDownload}
          linkText={localLL.cards.cli.download()}
          logo={<DefguardIcon />}
          selected={choice === AddStandaloneDeviceModalChoice.CLI}
          onSelect={() => handleChange(AddStandaloneDeviceModalChoice.CLI)}
        />
        <DeviceSetupMethodCard
          title={localLL.cards.manual.title()}
          subtitle={localLL.cards.manual.subtitle()}
          logo={<SvgWireguardLogo />}
          selected={choice === AddStandaloneDeviceModalChoice.MANUAL}
          onSelect={() => handleChange(AddStandaloneDeviceModalChoice.MANUAL)}
        />
      </div>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
          onClick={() => close()}
          size={ButtonSize.LARGE}
        />
        <Button
          loading={
            networks === undefined ||
            availableIpLoading ||
            availableIpResponse === undefined
          }
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.next()}
          onClick={() => {
            if (availableIpResponse) {
              handleNext();
            } else {
              void refetchAvailableIp();
            }
          }}
        />
      </div>
    </div>
  );
};

const DefguardIcon = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={211}
      height={53}
      viewBox="0 0 211 53"
      fill="none"
    >
      <g clipPath="url(#clip0_9115_716)">
        <path
          d="M206.451 7.85004V20.6284H206.161C204.868 18.177 202.301 16.4783 199.348 16.4783H196.491C192.225 16.4783 188.77 19.9335 188.77 24.1994V31.9398C188.77 36.2057 192.225 39.6609 196.491 39.6609H199.348C202.262 39.6609 204.791 38.0201 206.104 35.6266H206.316L206.47 39.2362H209.25V7.85004H206.47H206.451ZM200.351 36.7655H197.495C194.29 36.7655 191.704 34.1789 191.704 30.9747V25.1838C191.704 21.9796 194.29 19.393 197.495 19.393H200.351C203.556 19.393 206.142 21.9796 206.142 25.1838V30.9747C206.142 34.1789 203.556 36.7655 200.351 36.7655Z"
          fill="#222222"
        />
        <path
          d="M177.844 16.9029H176.59C175.528 16.9029 174.66 17.7715 174.66 18.8332V39.2554H177.844V19.4315H186.531V16.9029H177.844Z"
          fill="#222222"
        />
        <path
          d="M167.286 39.2554L167.247 23.3886C167.247 19.586 164.159 16.4783 160.337 16.4783H156.573C152.77 16.4783 149.662 19.5667 149.662 23.3886H152.539C152.809 20.8986 154.893 18.9683 157.461 18.9683H159.43C162.171 18.9683 164.41 21.1881 164.41 23.9484V25.898H155.839C152.037 25.898 148.929 28.9864 148.929 32.8083C148.929 36.6303 152.017 39.7187 155.839 39.7187H158.619C160.916 39.7187 162.923 38.5798 164.178 36.8619H164.39V39.294H167.247L167.286 39.2554ZM158.696 37.1901H156.38C153.948 37.1901 151.998 35.2212 151.998 32.8083C151.998 30.3955 153.967 28.4266 156.38 28.4266H164.429V31.4765C164.429 34.6421 161.862 37.2094 158.696 37.2094V37.1901Z"
          fill="#222222"
        />
        <path
          d="M140.706 30.9746C140.706 34.1788 138.12 36.7654 134.915 36.7654H133.448C130.244 36.7654 127.658 34.1788 127.658 30.9746V16.9029H124.724V31.959C124.724 36.2249 128.179 39.6801 132.445 39.6801H133.931C136.846 39.6801 139.374 38.0393 140.687 35.6458H140.899L141.054 39.2554H143.833V16.9029H140.764L140.725 30.9746H140.706Z"
          fill="#222222"
        />
        <path
          d="M115.014 35.0861H105.093C104.147 35.0861 103.394 34.4106 103.394 33.4647C103.394 32.5189 104.147 31.7661 105.093 31.7661H107.853H111.173C114.898 31.7661 117.929 28.7356 117.929 25.0101V23.2343C117.929 21.0917 116.906 19.2 115.362 17.9647V16.4011H120.033V13.8725H115.13C114.068 13.8725 113.2 14.7411 113.2 15.8028V16.8258C112.563 16.6135 111.887 16.4784 111.173 16.4784H107.853C104.127 16.4784 101.097 19.5089 101.097 23.2343V25.0101C101.097 27.2879 102.236 29.2953 103.954 30.5114C102.197 30.5886 100.788 32.017 100.788 33.7929C100.788 35.2599 101.772 36.4566 103.104 36.8427C100.865 37.5376 99.2438 39.41 99.2438 41.6298V42.2667C99.2438 45.0849 101.83 47.3627 105.035 47.3627H114.976C118.18 47.3627 120.766 44.7761 120.766 41.5719V40.8384C120.766 37.6341 118.18 35.0475 114.976 35.0475L115.014 35.0861ZM104.031 22.6938C104.031 20.6477 105.768 18.9877 107.891 18.9877H111.192C113.315 18.9877 115.053 20.6477 115.053 22.6938V25.5506C115.053 27.5967 113.315 29.2567 111.192 29.2567H107.891C105.768 29.2567 104.031 27.5967 104.031 25.5506V22.6938ZM114.879 44.6217H105.208C103.336 44.6217 101.811 43.0968 101.811 41.2244C101.811 39.352 103.336 37.8271 105.208 37.8271H114.879C116.751 37.8271 118.276 39.352 118.276 41.2244C118.276 43.0968 116.751 44.6217 114.879 44.6217Z"
          fill="#222222"
        />
        <path
          d="M96.445 10.398V7.85004H89.9593C88.8977 7.85004 88.0291 8.71866 88.0291 9.78031V16.903H84.1492V19.5089H88.0291V39.2555H90.8666V19.5089H95.9432V16.903H90.8666V10.398H96.445Z"
          fill="#222222"
        />
        <path
          d="M78.5129 28.7162H81.2346V26.1296C81.2346 20.8021 76.9108 16.4783 71.5832 16.4783H70.6953C65.3677 16.4783 61.0439 20.8021 61.0439 26.1296V29.5076C61.0439 35.1054 65.6187 39.6608 71.1972 39.6608H72.0851C76.4861 39.6608 80.1536 36.7075 81.3118 32.6925H77.9917C76.737 35.8582 73.8609 37.9429 70.0969 37.0742C66.3329 36.2056 64.0745 32.8276 64.0745 29.1408V28.7162H78.5129ZM71.2937 18.8718C75.27 18.8718 78.1268 22.0954 78.1268 26.091V26.2647H64.0745V26.091C64.0745 22.1147 67.298 18.8718 71.2937 18.8718Z"
          fill="#222222"
        />
        <path
          d="M52.5893 7.85004V20.6284H52.2998C51.0065 18.177 48.4393 16.4783 45.486 16.4783H42.6292C38.3633 16.4783 34.9081 19.9335 34.9081 24.1994V31.9398C34.9081 36.2057 38.3633 39.6609 42.6292 39.6609H45.486C48.4007 39.6609 50.9293 38.0201 52.2419 35.6266H52.4542L52.6086 39.2362H55.3882V7.85004H52.6086H52.5893ZM46.4897 36.7655H43.6329C40.4286 36.7655 37.8421 34.1789 37.8421 30.9747V25.1838C37.8421 21.9796 40.4286 19.393 43.6329 19.393H46.4897C49.6939 19.393 52.2805 21.9796 52.2805 25.1838V30.9747C52.2805 34.1789 49.6939 36.7655 46.4897 36.7655Z"
          fill="#222222"
        />
        <path
          d="M22.516 0.128967V11.419L12.743 5.77399L0.163208 13.0362V40.4578L12.7329 47.72L22.5059 42.075V47.8929L17.777 50.629L20.5736 52.2462L25.3025 49.5102V25.9333L12.7329 18.6711L2.95985 24.3161V14.6433L12.7329 8.99826L22.5059 14.6433V17.8574L25.3025 19.4746V1.74619L22.5059 0.128967H22.516ZM2.95985 38.8406V29.1678L12.7329 34.8128L22.5059 29.1678V38.8406L12.7329 44.4856L2.95985 38.8406ZM21.1126 26.747L12.7329 31.5885L4.35309 26.747L12.7329 21.9055L21.1126 26.747Z"
          fill="#0C8CE0"
        />
      </g>
      <defs>
        <clipPath id="clip0_9115_716">
          <rect
            width={210.399}
            height={52.1173}
            fill="white"
            transform="translate(0.163208 0.128967)"
          />
        </clipPath>
      </defs>
    </svg>
  );
};
