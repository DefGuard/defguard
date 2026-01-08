import { useSuspenseQuery } from '@tanstack/react-query';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { getOpenIdClientQueryOptions } from '../../shared/query';
import { CeOpenIdClientModal } from './modals/CEOpenIdClientModal/CEOpenIdClientModal';
import { OpenIdClientTable } from './OpenIdTable';

export const OpenIdPage = () => {
  const { data } = useSuspenseQuery(getOpenIdClientQueryOptions);
  return (
    <>
      <Page title={m.openid_title()} id="openid-page">
        <SizedBox height={ThemeSpacing.Xl3} />
        <OpenIdClientTable data={data} />
      </Page>
      <CeOpenIdClientModal />
    </>
  );
};
