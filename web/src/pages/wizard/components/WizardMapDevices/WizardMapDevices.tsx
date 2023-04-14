import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
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
import { ImportedDevice } from '../../../../shared/types';
import { useWizardStore } from '../../hooks/useWizardStore';
import { MapDeviceRow } from './components/MapDeviceRow';

export const WizardMapDevices = () => {
  const { LL } = useI18nContext();
  const {
    network: { createUserDevices },
  } = useApi();
  const toaster = useToaster();
  const [submitSubject, nextStepSubject, setWizardState] = useWizardStore(
    (state) => [state.submitSubject, state.nextStepSubject, state.setState],
    shallow
  );
  const devices = useWizardStore((state) => state.importedNetworkDevices);
  const {
    user: { getUsers },
  } = useApi();

  const { isLoading, data: users } = useQuery([QueryKeys.FETCH_USERS], getUsers, {
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });
  const { isLoading: createLoading, mutate } = useMutation(createUserDevices, {
    onSuccess: () => {
      setWizardState({ disableNext: false });
      toaster.success(LL.wizard.deviceMap.crateSuccess());
      nextStepSubject.next();
    },
    onError: (err) => {
      setWizardState({ disableNext: false });
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

  const renderRow = useCallback(
    (device: ImportedDevice) => (
      <MapDeviceRow options={getUsersOptions} device={device} />
    ),
    [getUsersOptions]
  );

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      if (devices) {
        const deviceWithoutUser = devices?.find((d) => d.user_id === -1);
        if (deviceWithoutUser) {
          toaster.error('Please assign all remaining devices.');
        } else {
          setWizardState({ disableNext: true });
          mutate({ devices });
        }
      }
    });
    return () => sub?.unsubscribe();
  }, [devices, mutate, setWizardState, submitSubject, toaster]);

  if (isLoading || !devices || createLoading) return <LoaderSpinner />;

  return (
    <Card id="wizard-map-devices">
      <VirtualizedList<ImportedDevice>
        customRowRender={renderRow}
        data={devices}
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
