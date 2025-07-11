import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { type ReactNode, useMemo, useState } from 'react';
import { upperCaseFirst } from 'text-case';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ListCellTags } from '../../../../../../shared/components/Layout/ListCellTags/ListCellTags';
import { ListCellText } from '../../../../../../shared/components/Layout/ListCellText/ListCellText';
import { ListHeader } from '../../../../../../shared/components/Layout/ListHeader/ListHeader';
import type { ListHeaderColumnConfig } from '../../../../../../shared/components/Layout/ListHeader/types';
import { CheckBox } from '../../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { InteractionBox } from '../../../../../../shared/defguard-ui/components/Layout/InteractionBox/InteractionBox';
import { NoData } from '../../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { ListSortDirection } from '../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import type { AclAlias } from '../../../../types';
import { DividerHeader } from '../../shared/DividerHeader';
import type { AclAliasListData } from '../types';
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
        label: headersLL.rules(),
        key: 'rules',
        enabled: false,
      },
      {
        label: headersLL.status(),
        key: 'status',
        enabled: false,
      },
      {
        label: headersLL.kind(),
        sortKey: 'kind',
        enabled: true,
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
                  <div className="cell name">
                    <ListCellText text={upperCaseFirst(alias.name)} />
                  </div>
                  <div className="cell ip">
                    <ListCellTags data={alias.display.destination} />
                  </div>
                  <div className="cell ports">
                    <ListCellTags
                      data={alias.display.ports}
                      placeholder={LL.acl.fieldsSelectionLabels.ports()}
                    />
                  </div>
                  <div className="cell protocols">
                    <ListCellTags
                      data={alias.display.protocols}
                      placeholder={LL.acl.fieldsSelectionLabels.protocols()}
                    />
                  </div>
                  <div className="cell rules">
                    <ListCellTags data={alias.display.rules} />
                  </div>
                  <div className="cell status">
                    <AclAliasStatusDisplay status={alias.state} />
                  </div>
                  <div className="cell kind">
                    <ListCellText text={upperCaseFirst(alias.kind)} />
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
