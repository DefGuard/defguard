import './style.scss';

import { ExpandableSection } from '../../shared/components/Layout/ExpandableSection/ExpandableSection';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';

export const OverviewIndexPage = () => {
  return (
    <PageContainer id="overview-index">
      <div>
        <header>
          <h1>All locations overview</h1>
          <div className="controls"></div>
        </header>
        <ExpandableSection textAs="h2" text="All locations summary">
          <p>Summary indeed</p>
        </ExpandableSection>
      </div>
    </PageContainer>
  );
};
