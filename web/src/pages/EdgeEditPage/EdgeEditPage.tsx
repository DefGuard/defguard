import { Link } from '@tanstack/react-router';
import { useNavigate, useParams } from '@tanstack/react-router';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { useSuspenseQuery } from '@tanstack/react-query';
import { getEdgeQueryOptions } from '../../shared/query';

const breadcrumbsLinks = [
  <Link
    to="/settings"
    search={{
      tab: 'notifications',
    }}
    key={0}
  >
    Notifications
  </Link>,
  <Link key={1} to="/settings/smtp">
    SMTP Configuration
  </Link>,
];

export const EdgeEditPage = () => {
  const { edgeId: paramsId } = useParams({
    from: '/_authorized/_default/edge/$edgeId/edit',
  });
  const { data: edge } = useSuspenseQuery(getEdgeQueryOptions(Number(paramsId)));
  return (
    <EditPage
      pageTitle="Edge component"
      links={breadcrumbsLinks}
      headerProps={{ title: 'Edit Edge component' }}
    >
      ID: {paramsId} name: {edge.name}
    </EditPage>
  );
};
