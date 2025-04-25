import './style.scss';

import { useState } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { PageLimiter } from '../../shared/components/Layout/PageLimiter/PageLimiter';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { ListItemCount } from '../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { Search } from '../../shared/defguard-ui/components/Layout/Search/Search';
import { ActivityList } from './components/ActivityList';
import { useActivityMock } from './useActivityMock';

export const ActivityPage = () => {
  return (
    <PageContainer id="activity-page">
      <PageLimiter>
        <PageContent />
      </PageLimiter>
    </PageContainer>
  );
};

const PageContent = () => {
  const [search, setSearch] = useState('');
  const { LL } = useI18nContext();
  const data = useActivityMock();

  return (
    <>
      <header className="page-header">
        <h1>Activity</h1>
        <Search
          placeholder={LL.common.search()}
          onDebounce={(val) => {
            setSearch(val);
          }}
        />
      </header>
      <div id="activity-list">
        <div className="top">
          <h2>All activity</h2>
          <ListItemCount count={10} />
          <div className="controls"></div>
        </div>
        <Card id="activity-list-card">
          <ActivityList data={data} />
        </Card>
      </div>
    </>
  );
};
