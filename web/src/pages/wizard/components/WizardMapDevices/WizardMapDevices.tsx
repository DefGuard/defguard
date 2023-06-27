import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useMutation } from 'wagmi';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/components/layout/Card/Card';
import LoaderSpinner from '../../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import { SelectOption } from '../../../../shared/components/layout/Select/Select';
import {
  ListHeader,
  VirtualizedList,
} from '../../../../shared/components/layout/VirtualizedList/VirtualizedList';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { ImportedDevice, MappedDevice } from '../../../../shared/types';
import { useWizardStore } from '../../hooks/useWizardStore';
import { MapDeviceRow } from './components/MapDeviceRow';

export const WizardMapDevices = () => {
  const [mappedDevices, setMappedDevices] = useState<DeviceRowData[]>([]);
  const { LL } = useI18nContext();
  const {
    network: { mapUserDevices: createUserDevices },
  } = useApi();
  const toaster = useToaster();
  const [submitSubject, nextStepSubject, setWizardState] = useWizardStore(
    (state) => [state.submitSubject, state.nextStepSubject, state.setState],
    shallow
  );
  const importedDevices = useWizardStore((state) => state.importedNetworkDevices);
  const importedNetwork = useWizardStore((state) => state.importedNetworkConfig);
  const {
    user: { getUsers },
  } = useApi();

  const { isLoading, data: users } = useQuery([QueryKeys.FETCH_USERS], getUsers, {
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });
  const { isLoading: createLoading, mutate } = useMutation(createUserDevices, {
    onSuccess: () => {
      setWizardState({ loading: false });
      toaster.success(LL.wizard.deviceMap.crateSuccess());
      nextStepSubject.next();
    },
    onError: (err) => {
      setWizardState({ loading: false });
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const getUsersOptions = useMemo(
    (): SelectOption<number>[] =>
      users?.map((user) => ({
        value: user.id as number,
        label: `${user.first_name} ${user.last_name}`,
        key: user.id as number,
        meta: ``,
      })) ?? [],
    [users]
  );

  const getHeaders = useMemo(
    (): ListHeader[] => [
      { text: 'Device Name', key: 0, sortable: false },
      { text: 'IP', key: 1, sortable: false },
      { text: 'User', key: 2, sortable: false },
    ],
    []
  );

  const handleDeviceChange = useCallback(
    (device: MappedDevice) => {
      const clone = [...mappedDevices];
      const deviceIndex = clone.findIndex(
        (d) => d.wireguard_pubkey === device.wireguard_pubkey
      );
      if (!isUndefined(deviceIndex)) {
        clone[deviceIndex] = device;
        setMappedDevices(clone);
      }
    },
    [mappedDevices]
  );

  const renderRow = useCallback(
    (device: DeviceRowData, index?: number) => (
      <MapDeviceRow
        options={getUsersOptions}
        device={device}
        testId={`map-device-${index}`}
        onChange={handleDeviceChange}
      />
    ),
    [getUsersOptions, handleDeviceChange]
  );

  const handleSubmit = useCallback(() => {
    if (mappedDevices.length && importedNetwork?.id) {
      const deviceWithoutUser = mappedDevices?.find((d) => isUndefined(d.user_id));
      if (deviceWithoutUser) {
        toaster.error('Please assign all remaining devices.');
      } else {
        setWizardState({ loading: true });
        mutate({
          devices: mappedDevices as MappedDevice[],
          networkId: importedNetwork.id,
        });
      }
    }
  }, [importedNetwork?.id, mappedDevices, mutate, setWizardState, toaster]);

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      handleSubmit();
    });
    return () => sub?.unsubscribe();
  }, [handleSubmit, submitSubject]);

  useEffect(() => {
    if (importedDevices) {
      const res: DeviceRowData[] = importedDevices.map((d) => ({
        user_id: undefined,
        wireguard_ip: d.wireguard_ip,
        wireguard_pubkey: d.wireguard_pubkey,
      }));
      setMappedDevices(res);
    }
  }, [importedDevices]);

  if (isLoading || !importedDevices || createLoading) return <LoaderSpinner />;

  return (
    <Card id="wizard-map-devices" shaded>
      <VirtualizedList<DeviceRowData>
        customRowRender={renderRow}
        data={mappedDevices}
        rowSize={70}
        headers={getHeaders}
        headerPadding={{
          left: 20,
          right: 20,
        }}
        padding={{
          left: 47,
          right: 47,
        }}
      />
    </Card>
  );
};

export type DeviceRowData = ImportedDevice & {
  user_id?: number;
};
