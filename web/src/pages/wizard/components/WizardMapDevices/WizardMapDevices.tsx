import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
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
import { useWizardStore } from '../../hooks/useWizardStore';
import { MapDeviceRow } from './components/MapDeviceRow';

type WizardMapFormDevice = {
  name?: string;
  user_id?: number;
  wireguard_ip: string;
  wireguard_pubkey: string;
};

export type WizardMapFormValues = {
  devices: WizardMapFormDevice[];
};

export const WizardMapDevices = () => {
  const submitElementRef = useRef<HTMLInputElement | null>(null);
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
  const {
    user: { getUsers },
  } = useApi();

  const schema = useMemo(() => {
    return yup.object().shape({
      devices: yup.array().of(
        yup.object().shape({
          wireguard_ip: yup.string().required(),
          user_id: yup.number().required(),
          wireguard_pubkey: yup.string().required(),
          name: yup.string().required(),
        })
      ),
    });
  }, []);

  const { isLoading, data: users } = useQuery([QueryKeys.FETCH_USERS], getUsers, {
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });

  const { isLoading: createLoading } = useMutation(createUserDevices, {
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

  const defaultValues = useMemo(() => {
    if (importedDevices && importedDevices.length) {
      const devices = importedDevices.map((d) => ({
        user_id: -1,
        name: d.wireguard_pubkey,
        wireguard_pubkey: d.wireguard_pubkey,
        wireguard_ip: d.wireguard_ip,
      }));
      return {
        devices,
      };
    }
    return undefined;
  }, [importedDevices]);

  const { handleSubmit, control, reset } = useForm<WizardMapFormValues>({
    defaultValues,
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
    (_: WizardMapFormDevice, index?: number) => (
      <MapDeviceRow options={getUsersOptions} control={control} index={index as number} />
    ),
    [control, getUsersOptions]
  );

  const handleValidSubmit: SubmitHandler<WizardMapFormValues> = (values) => {
    console.log(values);
  };

  // allows to submit form from WizardNav
  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      if (submitElementRef.current) {
        submitElementRef.current.click();
      }
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

  useEffect(() => {
    if (defaultValues) {
      reset(defaultValues);
    }
  }, [defaultValues, reset]);

  if (isLoading || !importedDevices || createLoading) return <LoaderSpinner />;

  return (
    <Card id="wizard-map-devices" shaded>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <VirtualizedList
          customRowRender={renderRow}
          data={defaultValues?.devices ?? []}
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
