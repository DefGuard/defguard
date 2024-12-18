import './style.scss';

import dayjs from 'dayjs';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { DeviceAvatar } from '../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { LimitedText } from '../../../../shared/defguard-ui/components/Layout/LimitedText/LimitedText';
import {
  ListHeader,
  ListSortDirection,
} from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { MockDevice, useDevicesPage } from '../../hooks/useDevicesPage';

export const DevicesList = () => {
  const { LL } = useI18nContext();
  const localLL = LL.devicesPage.list;
  const labels = localLL.columns.labels;
  const [{ devices, search }] = useDevicesPage();

  const renderRow = useCallback(
    (device: MockDevice) => <DeviceRow key={device.id} {...device} />,
    [],
  );

  const listHeaders = useMemo(
    (): ListHeader[] => [
      {
        key: 0,
        text: labels.name(),
        active: true,
        sortable: true,
        sortDirection: ListSortDirection.DESC,
      },
      { key: 1, text: labels.location() },
      { key: 2, text: labels.assignedIp() },
      { key: 3, text: labels.description() },
      { key: 4, text: labels.addedBy() },
      { key: 5, text: labels.addedAt() },
      { key: 6, text: labels.edit() },
    ],
    [labels],
  );

  const dataAfterFilter = useMemo(
    () =>
      devices.filter((d) => d.name.toLowerCase().includes(search.trim().toLowerCase())),
    [devices, search],
  );

  return (
    <VirtualizedList
      id="devices-page-devices-list"
      data={dataAfterFilter}
      rowSize={70}
      customRowRender={renderRow}
      headers={listHeaders}
      headerPadding={{
        left: 15,
        right: 15,
      }}
      padding={{
        left: 70,
        right: 70,
      }}
    />
  );
};

const DeviceRow = ({
  addedBy,
  addedDate,
  assignedIp,
  description,
  id,
  location,
  name,
}: MockDevice) => {
  const formatDate = useMemo(() => {
    const day = dayjs(addedDate);
    return day.format('DD.MM.YYYY | HH:mm');
  }, [addedDate]);
  return (
    <div className="device-row">
      <div className="cell-1">
        <DeviceAvatar deviceId={id} />
        <LimitedText floatingClassName="device-item-floating" text={name} />
      </div>
      <div className="cell-2">
        <LimitedText
          floatingClassName="device-item-floating"
          text={location[0]?.name ?? ''}
        />
      </div>
      <div className="cell-3">
        <span>{assignedIp}</span>
      </div>
      <div className="cell-4">
        <LimitedText floatingClassName="device-item-floating" text={description} />
      </div>
      <div className="cell-5">
        <LimitedText floatingClassName="device-item-floating" text={addedBy} />
      </div>
      <div className="cell-6">
        <span>{formatDate}</span>
      </div>
      <div className="cell-7">
        <DeviceRowEditButton />
      </div>
    </div>
  );
};

const DeviceRowEditButton = () => {
  const { LL } = useI18nContext();
  return (
    <EditButton>
      <EditButtonOption text={LL.common.controls.edit()} />
      <EditButtonOption
        text={LL.common.controls.delete()}
        styleVariant={EditButtonOptionStyleVariant.WARNING}
      />
    </EditButton>
  );
};
