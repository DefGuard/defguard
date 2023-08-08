import './style.scss';

import { useI18nContext } from '../../../i18n/i18n-react';
import {
  ActivityStatus,
  ActivityType,
} from '../../../shared/defguard-ui/components/Layout/ActivityStatus/ActivityStatus';

export const OverviewActivityStream = () => {
  const { LL } = useI18nContext();
  return (
    <div className="activity-stream">
      <header>
        <h2>{LL.activityOverview.header()}</h2>
      </header>
      <p className="no-data-text">{LL.activityOverview.noData()}</p>
      <div className="stream">
        <div className="activity">
          <div className="info">
            <ActivityStatus connectionStatus={ActivityType.ALERT} />
            <span className="time">15:31</span>
          </div>
          <p className="message">John Goodman uses 37% of network capacity</p>
        </div>
      </div>
    </div>
  );
};
