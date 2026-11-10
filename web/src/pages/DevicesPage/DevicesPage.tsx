import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { DevicesOverviewTable } from './components/DevicesOverviewTable/DevicesOverviewTable';

export const DevicesPage = () => {
  return (
    <Page id="devices-page" title={m.devices_title()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      <TablePageLayout>
        <DevicesOverviewTable />
      </TablePageLayout>
    </Page>
  );
};
