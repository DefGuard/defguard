import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { ReactNode, useMemo, useState } from 'react';
import { upperCaseFirst } from 'text-case';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ListHeader } from '../../../../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../../../../shared/components/Layout/ListHeader/types';
import { CheckBox } from '../../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { InteractionBox } from '../../../../../../shared/defguard-ui/components/Layout/InteractionBox/InteractionBox';
import { NoData } from '../../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { ListSortDirection } from '../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { AclAlias } from '../../../../types';
import { DividerHeader } from '../../shared/DividerHeader';
import { RenderTagDisplay } from '../../shared/RenderTagDisplay';
import { AclAliasListData } from '../types';
import { AclAliasStatusDisplay } from './AclAliasStatus/AclAliasStatus';
import { AliasEditButton } from './AliasEditButton';

type AliasesListProps = {
  data: AclAliasListData[];
  header: {
    text: string;
    extras?: ReactNode;
  };
  noDataMessage: string;
  isAppliedList?: boolean;
  selected?: Record<number, boolean | undefined>;
  allSelected?: boolean;
  onSelect?: (key: number, value: boolean) => void;
  onSelectAll?: (value: boolean, state: Record<number, boolean | undefined>) => void;
};

export const AliasesList = ({
  data,
  header,
  noDataMessage,
  selected,
  allSelected,
  onSelect,
  onSelectAll,
}: AliasesListProps) => {
  const { LL } = useI18nContext();
  const headersLL = LL.acl.listPage.aliases.list.headers;
  const [sortKey, setSortKey] = useState<keyof AclAlias>('name');
  const [sortDir, setSortDir] = useState<ListSortDirection>(ListSortDirection.ASC);

  const selectionEnabled = useMemo(
    () =>
      isPresent(onSelect) &&
      isPresent(onSelectAll) &&
      isPresent(selected) &&
      isPresent(allSelected),
    [onSelect, onSelectAll, selected, allSelected],
  );

  const sortedAliases = useMemo(
    () => orderBy(data, [sortKey], [sortDir.valueOf().toLowerCase() as 'asc' | 'desc']),
    [data, sortDir, sortKey],
  );

  const listHeaders = useMemo(
    (): ListHeaderColumnConfig<AclAlias>[] => [
      {
        label: headersLL.name(),
        sortKey: 'name',
        enabled: true,
      },
      {
        label: headersLL.ip(),
        key: 'ip',
        enabled: false,
      },
      {
        label: headersLL.ports(),
        key: 'ports',
        enabled: false,
      },
      {
        label: headersLL.protocols(),
        key: 'protocols',
        enabled: false,
      },
      {
        label: headersLL.status(),
        key: 'status',
        enabled: false,
      },
      {
        label: headersLL.edit(),
        key: 'edit',
        enabled: false,
      },
    ],
    [headersLL],
  );

  return (
    <div className="aliases-list">
      <DividerHeader text={header.text}>{header.extras}</DividerHeader>
      {sortedAliases.length === 0 && (
        <NoData customMessage={noDataMessage} messagePosition="center" />
      )}
      {sortedAliases.length > 0 && (
        <div className="list-container">
          <div className={clsx('header-track')}>
            <ListHeader<AclAlias>
              headers={listHeaders}
              sortDirection={sortDir}
              activeKey={sortKey}
              selectAll={allSelected}
              onSelectAll={(val) => {
                if (selectionEnabled) {
                  onSelectAll?.(val, selected ?? {});
                }
              }}
              onChange={(key, dir) => {
                setSortKey(key);
                setSortDir(dir);
              }}
            />
          </div>
          <ul>
            {sortedAliases.map((alias) => {
              let aliasSelected = false;
              if (selected) {
                aliasSelected = selected[alias.id] ?? false;
              }
              return (
                <li
                  key={alias.id}
                  className={clsx('alias-row', {
                    selectable: selectionEnabled,
                  })}
                >
                  {!selectionEnabled && <div className="cell empty"></div>}
                  {selectionEnabled && (
                    <div className="cell select-cell">
                      <InteractionBox
                        onClick={() => {
                          onSelect?.(alias.id, !aliasSelected);
                        }}
                      >
                        <CheckBox value={aliasSelected} />
                      </InteractionBox>
                    </div>
                  )}
                  <div className="cell name">{upperCaseFirst(alias.name)}</div>
                  <div className="cell ip">
                    <RenderTagDisplay data={alias.display.destination} />
                  </div>
                  <div className="cell ports">
                    <RenderTagDisplay data={alias.display.ports} />
                  </div>
                  <div className="cell protocols">
                    <RenderTagDisplay data={alias.display.protocols} />
                  </div>
                  <div className="cell status">
                    <AclAliasStatusDisplay status={alias.state} />
                  </div>
                  <div className="cell edit">
                    <AliasEditButton alias={alias} />
                  </div>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
};
