import SvgIconActivityAdd from '../../svg/IconActivityAdd';
import SvgIconActivityRemoved from '../../svg/IconActivityRemoved';
import SvgIconActivityWarning from '../../svg/IconActivityWarning';
import SvgIconConnected from '../../svg/IconConnected';
import SvgIconDisconnected from '../../svg/IconDisconnected';
import { ActivityType } from '../ActivityStatus/ActivityStatus';

type Props = {
  status: ActivityType;
};

export const ActivityIcon = ({ status }: Props) => {
  switch (status) {
    case ActivityType.CONNECTED:
      return <SvgIconConnected />;
    case ActivityType.ADDED:
      return <SvgIconActivityAdd />;
    case ActivityType.ALERT:
      return <SvgIconActivityWarning />;
    case ActivityType.DISCONNECTED:
      return <SvgIconDisconnected />;
    case ActivityType.REMOVED:
      return <SvgIconActivityRemoved />;
    default:
      return null;
  }
};
