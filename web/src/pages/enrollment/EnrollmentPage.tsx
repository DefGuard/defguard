import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { MessageBox } from '../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../shared/defguard-ui/components/Layout/MessageBox/types';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { EnrollmentEmail } from './components/EnrollmentEmail/EnrollmentEmail';
import { EnrollmentVPN } from './components/EnrollmentVPN/EnrollmentVPN';
import { EnrollmentWelcomeMessage } from './components/EnrollmentWelcomeMessage/EnrollmentWelcomeMessage';
import { useEnrollmentStore } from './hooks/useEnrollmentStore';

export const EnrollmentPage = () => {
  const {
    settings: { getSettings },
  } = useApi();

  const { LL } = useI18nContext();

  const pageLL = LL.enrollmentPage;

  const setEnrollment = useEnrollmentStore((state) => state.setState);

  const { data: settingsData, isLoading } = useQuery({
    queryFn: getSettings,
    queryKey: [QueryKeys.FETCH_SETTINGS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    if (settingsData) {
      setEnrollment({ settings: settingsData });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsData]);

  return (
    <PageContainer id="enrollment-page">
      <h1>{pageLL.title()}</h1>
      <MessageBox type={MessageBoxType.WARNING} message={pageLL.messageBox()} />
      {!isLoading && (
        <div className="settings">
          <div className="left">
            <EnrollmentVPN />
            <EnrollmentWelcomeMessage />
          </div>
          <div className="right">
            <EnrollmentEmail />
          </div>
        </div>
      )}
    </PageContainer>
  );
};
