import './style.scss';

import saveAs from 'file-saver';
import { useEffect, useMemo, useState } from 'react';
import useBreakpoint from 'use-breakpoint';

import { AvatarBox } from '../../../../../shared/components/layout/AvatarBox/AvatarBox';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/components/layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/components/layout/EditButton/EditButtonOption';
import { Label } from '../../../../../shared/components/layout/Label/Label';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { displayDate } from '../../../../../shared/helpers/displayDate';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { Device } from '../../../../../shared/types';

interface Props {
  device: Device;
}

export const DeviceCard = ({ device }: Props) => {
  const toaster = useToaster();
  const user = useUserProfileV2Store((state) => state.user);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const setDeleteUserDeviceModal = useModalStore(
    (state) => state.setDeleteUserDeviceModal
  );
  const setUserDeviceModal = useModalStore((state) => state.setUserDeviceModal);
  const [editButtonVisible, setEditButtonVisible] = useState(
    breakpoint === 'mobile'
  );
  const {
    device: { downloadDeviceConfig },
  } = useApi();

  const handleDownload = () => {
    downloadDeviceConfig(device.id)
      .then((res) => {
        const blob = new Blob([res.replace(/^[^\S\r\n]+|[^\S\r\n]+$/gm, '')], {
          type: 'text/plain;charset=utf-8',
        });
        saveAs(blob, `${device.name.toLowerCase()}.conf`);
      })
      .catch((err) => {
        console.log(err);
        toaster.error('Clipboard is not accessible.');
      });
  };

  const formattedCreationDate = useMemo(
    () => displayDate(device.created),
    [device]
  );

  useEffect(() => {
    if (breakpoint === 'mobile' && editButtonVisible === false) {
      setEditButtonVisible(true);
    }

    if (breakpoint !== 'mobile' && editButtonVisible === true) {
      setEditButtonVisible(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint]);

  if (!user) return null;

  return (
    <Card
      className="device-card"
      onHoverStart={() => {
        if (breakpoint !== 'mobile') {
          setEditButtonVisible(true);
        }
      }}
      onHoverEnd={() => {
        if (breakpoint !== 'mobile') {
          setEditButtonVisible(false);
        }
      }}
    >
      <header>
        <AvatarBox>
          <DeviceAvatar />
        </AvatarBox>
        <h3 data-test="device-name">{device.name}</h3>
      </header>
      <div className="content">
        <div className="info">
          <Label>Last location</Label>
          <p data-text="device-location">Szczecin</p>
        </div>
        <div className="info">
          <Label>Last IP address</Label>
          <p>{device.wireguard_ip}</p>
        </div>
        <div className="info">
          <Label>Date added</Label>
          <p>{formattedCreationDate}</p>
        </div>
      </div>
      <EditButton visible={editButtonVisible}>
        <EditButtonOption
          text="Delete device"
          onClick={() =>
            setDeleteUserDeviceModal({ visible: true, device: device })
          }
        />
        <EditButtonOption
          text="Download config"
          onClick={() => handleDownload()}
        />
        <EditButtonOption
          text="Edit device"
          onClick={() => {
            setUserDeviceModal({
              visible: true,
              username: user.username,
              device: device,
            });
          }}
        />
      </EditButton>
    </Card>
  );
};
