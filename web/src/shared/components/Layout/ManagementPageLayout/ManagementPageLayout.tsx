import './style.scss';

import { clsx } from 'clsx';

import { useNavigationStore } from '../../../../components/Navigation/hooks/useNavigationStore';
import { Search } from '../../../defguard-ui/components/Layout/Search/Search';
import { ManagementPageProps } from './types';

export const ManagementPageLayout = ({
  children,
  title,
  actions,
  itemsCount,
  search,
}: ManagementPageProps) => {
  const navOpen = useNavigationStore((s) => s.isOpen);
  return (
    <div
      className={clsx('management-page', {
        'nav-open': navOpen,
      })}
    >
      <div className="page-content">
        <header>
          <h1>{title}</h1>
          {search && (
            <Search
              placeholder={search.placeholder}
              className="items-search"
              initialValue={undefined}
              debounceTiming={500}
              onDebounce={search.onSearch}
            />
          )}
        </header>
        <div className="top-bar">
          {itemsCount && (
            <div className="items-count">
              <p>{itemsCount.label}</p>
              {itemsCount.itemsCount !== undefined && (
                <div className="count-box">
                  <span>{itemsCount.itemsCount}</span>
                </div>
              )}
            </div>
          )}
          <div className="actions">{actions}</div>
        </div>
        <div className="list-container">{children}</div>
      </div>
    </div>
  );
};
