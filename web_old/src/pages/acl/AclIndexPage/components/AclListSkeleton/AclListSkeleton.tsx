import './style.scss';

import { range } from 'lodash-es';
import Skeleton from 'react-loading-skeleton';

export const AclListSkeleton = () => {
  return (
    <div className="acl-list-skeleton">
      {range(10).map((val) => (
        <Skeleton key={val} />
      ))}
    </div>
  );
};
