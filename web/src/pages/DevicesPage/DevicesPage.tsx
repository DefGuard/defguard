import { m } from '../../paraglide/messages';
import { AssignUserDeviceIPModal } from '../../shared/components/modals/AssignUserDeviceIPModal/AssignUserDeviceIPModal';
import { EditUserDeviceModal } from '../../shared/components/modals/EditUserDeviceModal/EditUserDeviceModal';
import { UserDeviceConfigModal } from '../../shared/components/modals/UserDeviceConfigModal/UserDeviceConfigModal';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { DevicesOverviewTable } from './components/DevicesOverviewTable/DevicesOverviewTable';

export const DevicesPage = () => {
  return (
    <>
      <Page id="devices-page" title={m.devices_title()}>
        <SizedBox height={ThemeSpacing.Xl3} />
        <TablePageLayout>
          <DevicesOverviewTable />
        </TablePageLayout>
      </Page>
      <EditUserDeviceModal />
      <UserDeviceConfigModal />
      <AssignUserDeviceIPModal />
    </>
  );
};
