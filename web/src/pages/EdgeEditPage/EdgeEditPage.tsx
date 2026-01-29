import { Link } from '@tanstack/react-router';
import { EditPage } from '../../shared/components/EditPage/EditPage';

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
  return (
    <EditPage
      pageTitle="Edit Edge component"
      links={breadcrumbsLinks}
      headerProps={{ title: 'Edit Edge component' }}
    >
      TODO
    </EditPage>
  );
};
