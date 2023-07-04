import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { useMutation } from 'wagmi';
import * as yup from 'yup';
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

export type WizardMapFormValues = {
  devices: ImportedDevice[];
};

export const WizardMapDevices = () => {
  const initialized = useRef(false);
  const submitElementRef = useRef<HTMLInputElement | null>(null);
  const { LL } = useI18nContext();
  const {
    network: { mapUserDevices },
  } = useApi();
  const toaster = useToaster();
  const setWizardState = useWizardStore((state) => state.setState);
  const setImportedDevices = useWizardStore((state) => state.setImportedDevices);
  const [submitSubject, nextStepSubject] = useWizardStore(
    (state) => [state.submitSubject, state.nextStepSubject],
    shallow
  );
  const importedDevices = useWizardStore((state) => state.importedNetworkDevices);
  const importedNetwork = useWizardStore((state) => state.importedNetworkConfig);
  const {
    user: { getUsers },
  } = useApi();

  const schema = useMemo(() => {
    return yup.object().shape({
      devices: yup.array().of(
        yup.object().shape({
          wireguard_ip: yup.string().required(),
          user_id: yup.number().required().min(1),
          wireguard_pubkey: yup.string().required(),
          name: yup.string().required(),
        })
      ),
    });
  }, []);

  const { isLoading, data: users } = useQuery([QueryKeys.FETCH_USERS_LIST], getUsers, {
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });

  const { isLoading: createLoading, mutate } = useMutation(mapUserDevices, {
    onSuccess: () => {
      setWizardState({ loading: false });
      toaster.success(LL.wizard.deviceMap.messages.crateSuccess());
      nextStepSubject.next();
    },
    onError: (err) => {
      setWizardState({ loading: false });
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { handleSubmit, control, reset, getValues } = useForm<WizardMapFormValues>({
    defaultValues: { devices: importedDevices ?? [] },
    resolver: yupResolver(schema),
    mode: 'onSubmit',
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

  const renderRow = useCallback(
    (data: DeviceRowData) => (
      <MapDeviceRow options={getUsersOptions} control={control} index={data.itemIndex} />
    ),
    [control, getUsersOptions]
  );

  const handleValidSubmit: SubmitHandler<WizardMapFormValues> = (values) => {
    if (importedNetwork) {
      setWizardState({ loading: true });
      mutate({
        devices: values.devices as MappedDevice[],
        networkId: importedNetwork.id,
      });
    }
  };

  const handleInvalidSubmit: SubmitErrorHandler<WizardMapFormValues> = () => {
    toaster.error(LL.wizard.deviceMap.messages.errorsInForm());
  };

  const devicesList = useMemo((): DeviceRowData[] => {
    if (importedDevices) {
      return importedDevices.map((_, index) => ({
        itemIndex: index,
      }));
    }

    return [];
  }, [importedDevices]);

  // allows to submit form from WizardNav
  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      if (submitElementRef.current) {
        submitElementRef.current.click();
      }
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

  // init form with values from imported config
  useEffect(() => {
    if (importedDevices && !initialized.current) {
      initialized.current = true;
      reset({ devices: importedDevices });
    }
  }, [importedDevices, reset]);

  // save form state so progress won't be lost
  useEffect(() => {
    const interval = setInterval(() => {
      const values = getValues();
      setImportedDevices(values.devices);
    }, 5000);

    return () => {
      clearInterval(interval);
    };
  }, [getValues, setImportedDevices]);

  if (isLoading || !importedDevices || createLoading) return <LoaderSpinner />;

  return (
    <Card id="wizard-map-devices" shaded>
      <form onSubmit={handleSubmit(handleValidSubmit, handleInvalidSubmit)}>
        <VirtualizedList<DeviceRowData>
          customRowRender={renderRow}
          data={devicesList}
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
        <input type="submit" className="visually-hidden" ref={submitElementRef} />
      </form>
    </Card>
  );
};

type DeviceRowData = {
  itemIndex: number;
};
