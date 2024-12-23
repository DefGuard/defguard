import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { PropsWithChildren, useEffect } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { ManagementPageLayout } from '../../shared/components/Layout/ManagementPageLayout/ManagementPageLayout';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { AddDeviceIcon } from './components/AddDeviceIcon';
import { DevicesList } from './components/DevicesList/DevicesList';
import { ConfirmDeviceDeleteModal } from './components/DevicesList/modals/ConfirmDeviceDeleteModal';
import { DevicesPageProvider, useDevicesPage } from './hooks/useDevicesPage';
import { AddStandaloneDeviceModal } from './modals/AddStandaloneDeviceModal/AddStandaloneDeviceModal';
import { useAddStandaloneDeviceModal } from './modals/AddStandaloneDeviceModal/store';
import { EditStandaloneModal } from './modals/EditStandaloneDeviceModal/EditStandaloneModal';
import { StandaloneDeviceConfigModal } from './modals/StandaloneDeviceConfigModal/StandaloneDeviceConfigModal';
import { StandaloneDeviceEnrollmentModal } from './modals/StandaloneDeviceEnrollmentModal/StandaloneDeviceEnrollmentModal';

export const DevicesPage = () => {
  return (
    <PageContext>
      <Page />
      {/* Add modals here */}
      <AddStandaloneDeviceModal />
      <ConfirmDeviceDeleteModal />
      <EditStandaloneModal />
      <StandaloneDeviceConfigModal />
      <StandaloneDeviceEnrollmentModal />
    </PageContext>
  );
};

const PageContext = (props: PropsWithChildren) => {
  return <DevicesPageProvider>{props.children}</DevicesPageProvider>;
};

const PageActions = () => {
  const { LL } = useI18nContext();
  const localLL = LL.devicesPage.bar.actions;
  const openStandaloneDeviceModal = useAddStandaloneDeviceModal((s) => s.open);
  return (
    <>
      <Button
        styleVariant={ButtonStyleVariant.PRIMARY}
        text={localLL.addNewDevice()}
        icon={<AddDeviceIcon />}
        onClick={() => openStandaloneDeviceModal()}
      />
    </>
  );
};

const Page = () => {
  const { LL } = useI18nContext();
  const localLL = LL.devicesPage;
  const [{ devices }, setPageState] = useDevicesPage();
  const currentUser = useAuthStore((s) => s.user);

  const {
    standaloneDevice: { getDevicesList },
    user: { getUser },
  } = useApi();

  const { data: userProfile } = useQuery({
    queryFn: () => getUser(currentUser?.username as string),
    queryKey: [QueryKeys.FETCH_USER_PROFILE, currentUser?.id],
    refetchOnMount: true,
    refetchOnReconnect: true,
    refetchOnWindowFocus: true,
    enabled: currentUser !== undefined,
  });

  const { data: devicesData } = useQuery({
    queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
    queryFn: getDevicesList,
    refetchOnMount: true,
    refetchOnReconnect: true,
    refetchOnWindowFocus: true,
  });

  useEffect(() => {
    if (userProfile && devicesData) {
      setPageState((s) => ({
        ...s,
        reservedDeviceNames: [
          ...userProfile.devices.map((d) => d.name.trim()),
          ...devicesData
            .filter((d) => d.added_by === (currentUser?.username as string))
            .map((d) => d.name.trim()),
        ],
        devices: devicesData,
      }));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userProfile, devicesData]);

  return (
    <ManagementPageLayout
      id="standalone-devices-page"
      title={localLL.title()}
      search={{
        placeholder: localLL.search.placeholder(),
        onSearch: (v) => {
          setPageState((s) => ({
            ...s,
            search: v,
          }));
        },
      }}
      actions={<PageActions />}
      itemsCount={{
        label: localLL.bar.itemsCount(),
        itemsCount: devices.length,
      }}
    >
      <DevicesList />
    </ManagementPageLayout>
  );
};
