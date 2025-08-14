import './style.scss';

import classNames from 'classnames';
import clsx from 'clsx';
import dayjs from 'dayjs';
import { isUndefined, orderBy } from 'lodash-es';
import type { TargetAndTransition } from 'motion/react';
import { useMemo, useState } from 'react';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ListCellTags } from '../../../../../shared/components/Layout/ListCellTags/ListCellTags';
import IconClip from '../../../../../shared/components/svg/IconClip';
import SvgIconCollapse from '../../../../../shared/components/svg/IconCollapse';
import SvgIconCopy from '../../../../../shared/components/svg/IconCopy';
import SvgIconExpand from '../../../../../shared/components/svg/IconExpand';
import { ColorsRGB } from '../../../../../shared/constants';
import { Badge } from '../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { LimitedText } from '../../../../../shared/defguard-ui/components/Layout/LimitedText/LimitedText';
import { ListCellText } from '../../../../../shared/defguard-ui/components/Layout/ListCellText/ListCellText';
import { NoData } from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';
import type { Device, DeviceNetworkInfo } from '../../../../../shared/types';
import { sortByDate } from '../../../../../shared/utils/sortByDate';
import type { ListCellTag } from '../../../../acl/AclIndexPage/components/shared/types';
import { useDeleteDeviceModal } from '../hooks/useDeleteDeviceModal';
import { useDeviceConfigModal } from '../hooks/useDeviceConfigModal';
import { useEditDeviceModal } from '../hooks/useEditDeviceModal';

const dateFormat = 'DD.MM.YYYY | HH:mm';

const formatDate = (date: string): string => {
  return dayjs.utc(date).format(dateFormat);
};

interface Props {
  device: Device;
  biometricEnabled: boolean;
  modifiable: boolean;
}

export const DeviceCard = ({ device, modifiable, biometricEnabled }: Props) => {
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile);
  const setDeleteDeviceModal = useDeleteDeviceModal((state) => state.setState);
  const setEditDeviceModal = useEditDeviceModal((state) => state.setState);
  const openDeviceConfigModal = useDeviceConfigModal((state) => state.open);
  const enterpriseSettings = useAppStore((state) => state.enterprise_settings);
  const { writeToClipboard } = useClipboard();

  const cn = useMemo(
    () =>
      classNames('device-card', {
        expanded,
      }),
    [expanded],
  );

  const getContainerAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      borderColor: ColorsRGB.White,
    };
    if (expanded || hovered) {
      res.borderColor = ColorsRGB.GrayBorder;
    }
    return res;
  }, [expanded, hovered]);

  // first, order by last_connected_at then if not preset, by network_id
  const orderedLocations = useMemo((): DeviceNetworkInfo[] => {
    const connected = device.networks.filter(
      (network) => !isUndefined(network.last_connected_at),
    );

    const neverConnected = device.networks.filter((network) =>
      isUndefined(network.last_connected_at),
    );

    const connectedSorted = sortByDate(
      connected,
      (n) => n.last_connected_at as string,
      true,
    );
    const neverConnectedSorted = orderBy(neverConnected, ['network_id'], ['desc']);

    return [...connectedSorted, ...neverConnectedSorted];
  }, [device.networks]);

  const latestLocation = orderedLocations.length ? orderedLocations[0] : undefined;

  if (!user) return null;

  return (
    <Card
      className={cn}
      initial={false}
      animate={getContainerAnimate}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <section className="main-info">
        <header
          className={clsx({
            biometry: biometricEnabled,
          })}
        >
          <DeviceAvatar deviceId={Number(device.id)} active={false} />
          {biometricEnabled && <IconBiometry />}
          <ListCellText testId="device-name" as="h3" text={device.name} />
        </header>
        <div className="section-content">
          <div className="limited">
            <Label>{LL.userPage.devices.card.labels.publicIP()}</Label>
            {latestLocation?.last_connected_ip && (
              <LimitedText
                text={latestLocation.last_connected_ip}
                testId="device-last-connected-from"
                otherContent={
                  <button
                    className="copy"
                    onClick={() => {
                      if (latestLocation.last_connected_ip) {
                        void writeToClipboard(latestLocation.last_connected_ip);
                      }
                    }}
                  >
                    <SvgIconCopy />
                  </button>
                }
              />
            )}
            {!latestLocation?.last_connected_ip && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
          <div className="limited">
            <Label>{LL.userPage.devices.card.labels.connectedThrough()}</Label>
            {latestLocation?.last_connected_at && (
              <LimitedText
                text={latestLocation?.network_name}
                otherContent={
                  <button
                    className="copy"
                    onClick={() => {
                      if (latestLocation.network_name) {
                        void writeToClipboard(latestLocation.network_name);
                      }
                    }}
                  >
                    <SvgIconCopy />
                  </button>
                }
              />
            )}
            {!latestLocation?.last_connected_at && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.connectionDate()}</Label>
            {latestLocation?.last_connected_at && (
              <p>{formatDate(latestLocation.last_connected_at)}</p>
            )}
            {!latestLocation?.last_connected_at && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
        </div>
      </section>
      <div className="locations">
        {orderedLocations.map((n) => (
          <DeviceLocation key={n.network_id} network_info={n} />
        ))}
      </div>
      <div className="card-controls">
        <EditButton visible={true}>
          <EditButtonOption
            text={LL.userPage.devices.card.edit.edit()}
            disabled={!modifiable}
            onClick={() => {
              setEditDeviceModal({
                visible: true,
                device: device,
              });
            }}
          />
          {!enterpriseSettings?.only_client_activation && (
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.STANDARD}
              text={LL.userPage.devices.card.edit.showConfigurations()}
              disabled={!device.networks?.length}
              onClick={() => {
                openDeviceConfigModal({
                  deviceName: device.name,
                  publicKey: device.wireguard_pubkey,
                  deviceId: device.id,
                  userId: user.user.id,
                  networks: device.networks.map((n) => ({
                    networkId: n.network_id,
                    networkName: n.network_name,
                  })),
                });
              }}
            />
          )}
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.devices.card.edit.delete()}
            disabled={!modifiable}
            onClick={() =>
              setDeleteDeviceModal({
                visible: true,
                device: device,
              })
            }
          />
        </EditButton>
        <ExpandButton
          expanded={expanded}
          onClick={() => setExpanded((state) => !state)}
        />
      </div>
    </Card>
  );
};

type DeviceLocationProps = {
  network_info: DeviceNetworkInfo;
};

const DeviceLocation = ({
  network_info: {
    network_id,
    network_name,
    network_gateway_ip,
    last_connected_ip,
    last_connected_at,
    device_wireguard_ips,
  },
}: DeviceLocationProps) => {
  const { LL } = useI18nContext();
  const { writeToClipboard } = useClipboard();
  const ipsTags = useMemo(
    (): ListCellTag[] =>
      device_wireguard_ips.map((ip) => ({
        key: ip,
        label: ip,
        displayAsTag: false,
      })),
    [device_wireguard_ips],
  );
  return (
    <div className="location" data-testid={`device-location-id-${network_id}`}>
      <header>
        <IconClip />
        <div className="info-wrapper">
          <ListCellText as="h3" testId="device-location-name" text={network_name} />
          {!isUndefined(network_gateway_ip) && <Badge text={network_gateway_ip} />}
        </div>
      </header>
      <div className="section-content">
        <div className="limited">
          <Label>{LL.userPage.devices.card.labels.lastLocation()}</Label>
          {last_connected_ip && (
            <LimitedText
              text={last_connected_ip}
              testId="device-last-connected-from"
              otherContent={
                <button
                  className="copy"
                  onClick={() => {
                    void writeToClipboard(last_connected_ip);
                  }}
                >
                  <SvgIconCopy />
                </button>
              }
            />
          )}
          {!last_connected_ip && (
            <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
          )}
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
          {last_connected_at && (
            <p data-testid="device-last-connected-at">{formatDate(last_connected_at)}</p>
          )}
          {!last_connected_at && (
            <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
          )}
        </div>
        <div className="limited">
          <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
          <ListCellTags data={ipsTags} />
        </div>
      </div>
    </div>
  );
};

type ExpandButtonProps = {
  expanded: boolean;
  onClick: () => void;
};

const ExpandButton = ({ expanded, onClick }: ExpandButtonProps) => {
  return (
    <button className="device-card-expand" onClick={onClick}>
      {expanded ? <SvgIconCollapse /> : <SvgIconExpand />}
    </button>
  );
};

const IconBiometry = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="61"
      height="60"
      viewBox="0 0 61 60"
      fill="none"
      className="biometry-icon"
    >
      <path
        d="M47.9486 7.41C47.7085 7.41 47.4685 7.35 47.2584 7.23C41.4972 4.26 36.5162 3 30.545 3C24.6038 3 18.9626 4.41 13.8316 7.23C13.1114 7.62 12.2113 7.35 11.7912 6.63C11.4011 5.91 11.6711 4.98 12.3913 4.59C17.9724 1.56 24.0937 0 30.545 0C36.9363 0 42.5175 1.41 48.6387 4.56C49.3889 4.95 49.6589 5.85 49.2688 6.57C48.9988 7.11 48.4887 7.41 47.9486 7.41ZM5.00979 23.16C4.70972 23.16 4.40966 23.07 4.13961 22.89C3.44947 22.41 3.29944 21.48 3.77953 20.79C6.75014 16.59 10.5309 13.29 15.0318 10.98C24.4538 6.12 36.5162 6.09 45.9682 10.95C50.4691 13.26 54.2499 16.53 57.2205 20.7C57.7006 21.36 57.5505 22.32 56.8604 22.8C56.1703 23.28 55.2401 23.13 54.76 22.44C52.0594 18.66 48.6387 15.69 44.5879 13.62C35.9761 9.21 24.9639 9.21 16.3821 13.65C12.3013 15.75 8.88058 18.75 6.18003 22.53C5.93998 22.95 5.48988 23.16 5.00979 23.16ZM23.7636 59.37C23.3735 59.37 22.9835 59.22 22.7134 58.92C20.1029 56.31 18.6926 54.63 16.6822 51C14.6117 47.31 13.5315 42.81 13.5315 37.98C13.5315 29.07 21.1531 21.81 30.515 21.81C39.8769 21.81 47.4985 29.07 47.4985 37.98C47.4985 38.82 46.8383 39.48 45.9982 39.48C45.158 39.48 44.4979 38.82 44.4979 37.98C44.4979 30.72 38.2266 24.81 30.515 24.81C22.8034 24.81 16.5321 30.72 16.5321 37.98C16.5321 42.3 17.4923 46.29 19.3227 49.53C21.2431 52.98 22.5634 54.45 24.8738 56.79C25.444 57.39 25.444 58.32 24.8738 58.92C24.5438 59.22 24.1537 59.37 23.7636 59.37ZM45.278 53.82C41.7073 53.82 38.5566 52.92 35.9761 51.15C31.5052 48.12 28.8347 43.2 28.8347 37.98C28.8347 37.14 29.4948 36.48 30.335 36.48C31.1751 36.48 31.8353 37.14 31.8353 37.98C31.8353 42.21 33.9957 46.2 37.6565 48.66C39.7869 50.1 42.2774 50.79 45.278 50.79C45.9982 50.79 47.1984 50.7 48.3987 50.49C49.2088 50.34 49.989 50.88 50.139 51.72C50.2891 52.53 49.7489 53.31 48.9088 53.46C47.1984 53.79 45.6981 53.82 45.278 53.82ZM39.2468 60C39.1268 60 38.9767 59.97 38.8567 59.94C34.0857 58.62 30.9651 56.85 27.6944 53.64C23.4936 49.47 21.1831 43.92 21.1831 37.98C21.1831 33.12 25.3239 29.16 30.425 29.16C35.526 29.16 39.6669 33.12 39.6669 37.98C39.6669 41.19 42.4574 43.8 45.9081 43.8C49.3589 43.8 52.1494 41.19 52.1494 37.98C52.1494 26.67 42.3974 17.49 30.395 17.49C21.8732 17.49 14.0716 22.23 10.5609 29.58C9.39068 32.01 8.79056 34.86 8.79056 37.98C8.79056 40.32 9.0006 44.01 10.801 48.81C11.101 49.59 10.711 50.46 9.93079 50.73C9.15063 51.03 8.28045 50.61 8.0104 49.86C6.5401 45.93 5.81995 42.03 5.81995 37.98C5.81995 34.38 6.51009 31.11 7.86037 28.26C11.8512 19.89 20.703 14.46 30.395 14.46C44.0478 14.46 55.15 24.99 55.15 37.95C55.15 42.81 51.0092 46.77 45.9081 46.77C40.8071 46.77 36.6663 42.81 36.6663 37.95C36.6663 34.74 33.8757 32.13 30.425 32.13C26.9743 32.13 24.1837 34.74 24.1837 37.95C24.1837 43.08 26.1641 47.88 29.7949 51.48C32.6454 54.3 35.376 55.86 39.6069 57.03C40.417 57.24 40.8671 58.08 40.6571 58.86C40.507 59.55 39.8769 60 39.2468 60Z"
        style={{ fill: 'var(--surface-main-primary)' }}
      />
    </svg>
  );
};
