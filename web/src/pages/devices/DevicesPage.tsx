import { PropsWithChildren, useEffect } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { ManagementPageLayout } from '../../shared/components/Layout/ManagementPageLayout/ManagementPageLayout';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../shared/defguard-ui/components/Layout/Button/types';
import { AddStandaloneDeviceModal } from '../overview/modals/AddStandaloneDeviceModal/AddStandaloneDeviceModal';
import { useAddStandaloneDeviceModal } from '../overview/modals/AddStandaloneDeviceModal/store';
import { AddDeviceIcon } from './components/AddDeviceIcon';
import { DevicesList } from './components/DevicesList/DevicesList';
import { ConfirmDeviceDeleteModal } from './components/DevicesList/modals/ConfirmDeviceDeleteModal';
import { DevicesPageProvider, useDevicesPage } from './hooks/useDevicesPage';
import { mockDevices } from './mock';

export const DevicesPage = () => {
  return (
    <PageContext>
      <Page />
      {/* Add modals here */}
      <AddStandaloneDeviceModal />
      <ConfirmDeviceDeleteModal />
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

  useEffect(() => {
    setPageState((s) => ({
      ...s,
      devices: mockDevices,
    }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <ManagementPageLayout
      title={localLL.title()}
      search={{
        placeholder: localLL.search.placeholder(),
        onSearch: (v) => {
          console.log(v);
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
