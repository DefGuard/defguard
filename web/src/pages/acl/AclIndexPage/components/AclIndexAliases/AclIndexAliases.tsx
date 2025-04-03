import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { upperCaseFirst } from 'text-case';
import { shallow } from 'zustand/shallow';

import { ListHeader } from '../../../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../../../shared/components/Layout/ListHeader/types';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { ListItemCount } from '../../../../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { NoData } from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Search } from '../../../../../shared/defguard-ui/components/Layout/Search/Search';
import { ListSortDirection } from '../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useAclCreateSelector } from '../../../acl-context';
import { AclAlias, AclAliasStatus } from '../../../types';
import { AlcAliasCEModal } from './modals/AlcAliasCEModal/AlcAliasCEModal';
import { useAclAliasCEModal } from './modals/AlcAliasCEModal/store';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import { AclAliasInfo } from '../../../../../shared/types';
import { AliasesList } from './components/AliasesList';

type ListTagDisplay = {
  key: string | number;
  label: string;
  displayAsTag?: boolean;
};

type AliasesFilters = {
  status: number[];
};

export type ListData = {
  context: {
    usedBy: ListTagDisplay[];
  };
} & AclAliasInfo;

const defaultFilters: AliasesFilters = {
  status: [],
};

export const AclIndexAliases = () => {
  const openCEModal = useAclAliasCEModal((s) => s.open, shallow);
  const aliases = useAclCreateSelector((s) => s.aliases);
  const itemCount = useMemo(() => aliases?.length ?? 0, [aliases?.length]);
  const [searchValue, setSearchValue] = useState('');
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases;

  const pendingAliasesCount = useMemo(() => {
    if (aliases) {
      return aliases.filter((alias) => alias.state !== AclAliasStatus.APPLIED).length;
    }
    return 0;
  }, [aliases]);

  const aliasesAfterSearch = useMemo(() => {
    if (aliases && searchValue) {
      return aliases.filter((rule) =>
        rule.name.trim().toLowerCase().includes(searchValue.toLowerCase().trim()),
      );
    }
    return aliases ?? [];
  }, [aliases, searchValue]);

  const pendingAliases = useMemo(
    () =>
      isPresent(aliases)
        ? prepareDisplay(
          aliasesAfterSearch,
          appliedFilters,
          true,
          aclContext,
        )
        : [],
    [aclContext, aliases, appliedFilters, localLL.list.tags, aliasesAfterSearch],
  );

  const [selectedPending, setSelectedPending] = useState<Record<number, boolean>>({});
  const handlePendingSelect = useCallback((key: number, value: boolean) => {
    setSelectedPending((s) => ({ ...s, [key]: value }));
  }, []);
  const handlePendingSelectAll = useCallback(
    (value: boolean, state: Record<number, boolean>) => {
      const newState = { ...state };
      for (const key in newState) {
        newState[key] = value;
      }
      setSelectedPending(newState);
    },
    [],
  );
  const pendingSelectionCount = useMemo(() => {
    let count = 0;
    for (const key in selectedPending) {
      if (selectedPending[key]) count++;
    }
    return count;
  }, [selectedPending]);

  const deployedRules = useMemo(() => {
    if (aliases) {
      return prepareDisplay(
        aliasesAfterSearch,
        appliedFilters,
        localLL.list.tags.allAllowed(),
        localLL.list.tags.allDenied(),
        false,
        aclContext,
      );
    }
    return [];
  }, [aclContext, aliases, appliedFilters, localLL.list.tags, aliasesAfterSearch]);

  const displayItemsCount = useMemo(
    () => deployedRules.length + pendingAliases.length,
    [deployedRules.length, pendingAliases.length],
  );

  const filters = useMemo(() => {
    const res: Record<string, FilterGroupsModalFilter> = {};
    const filterLL = localLL.modals.filterGroupsModal.groupHeaders;
    res.groups = {
      label: filterLL.groups(),
      order: 3,
      items: aclContext.groups.map((group) => ({
        label: group.name,
        searchValues: [group.name],
        value: group.id,
      })),
    };
    res.networks = {
      label: filterLL.location(),
      order: 1,
      items: aclContext.networks.map((network) => ({
        label: network.name,
        searchValues: [network.name],
        value: network.id,
      })),
    };
    res.aliases = {
      label: filterLL.alias(),
      order: 2,
      items: aclContext.aliases.map((alias) => ({
        label: alias.name,
        searchValues: [alias.name],
        value: alias.id,
      })),
    };

    res.status = {
      label: filterLL.status(),
      order: 4,
      items: [
        {
          label: ruleStatusLL.enabled(),
          value: 1000,
          searchValues: [ruleStatusLL.enabled()],
        },
        {
          label: ruleStatusLL.disabled(),
          value: 999,
          searchValues: [ruleStatusLL.disabled()],
        },
        {
          label: ruleStatusLL.new(),
          value: aclStatusToInt(AclStatus.NEW),
          searchValues: [ruleStatusLL.new()],
        },
        {
          label: ruleStatusLL.change(),
          value: aclStatusToInt(AclStatus.MODIFIED),
          searchValues: [ruleStatusLL.change()],
        },
        {
          label: ruleStatusLL.deployed(),
          value: aclStatusToInt(AclStatus.APPLIED),
          searchValues: [ruleStatusLL.deployed()],
        },
        {
          label: ruleStatusLL.delete(),
          value: aclStatusToInt(AclStatus.DELETED),
          searchValues: [ruleStatusLL.delete()],
        },
      ],
    };
    return res;
  }, [
    aclContext.groups,
    aclContext.networks,
    localLL.modals.filterGroupsModal.groupHeaders,
    ruleStatusLL,
  ]);

  const controlFilterDisplay = useMemo(() => {
    return appliedFiltersCount
      ? localLL.listControls.filter.applied({ count: appliedFiltersCount })
      : localLL.listControls.filter.nothingApplied();
  }, [appliedFiltersCount, localLL.listControls.filter]);

  const filtersPresent = appliedFiltersCount > 0;

  const applyText = useMemo(() => {
    if (pendingSelectionCount) {
      return localLL.listControls.apply.selective({ count: pendingSelectionCount });
    }
    if (pendingRulesCount) {
      return localLL.listControls.apply.all({ count: pendingRulesCount });
    }
    return localLL.listControls.apply.noChanges();
  }, [localLL.listControls.apply, pendingRulesCount, pendingSelectionCount]);

  // update or build selection state for list when rules are done loading
  useEffect(() => {
    if (aliases) {
      const pending = aliases.filter((rule) => rule.state !== AclStatus.APPLIED);
      const selectionEntries = Object.keys(selectedPending).length;
      if (selectionEntries !== pending.length) {
        const newSelectionState: Record<number, boolean> = {};
        for (const rule of pending) {
          newSelectionState[rule.id] = newSelectionState[rule.id] ?? false;
        }
        setSelectedPending(newSelectionState);
      }
    }
  }, [aliases, selectedPending]);

  return (
    <div id="acl-aliases">
      <header>
        <h2>Aliases</h2>
        <ListItemCount count={itemCount} />
        <Search
          placeholder="Find name"
          initialValue={searchValue}
          onDebounce={(searchChange) => {
            setSearchValue(searchChange);
          }}
        />
        <div className="controls">
          <Button
            className="filter"
            text="Filters"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
            icon={
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="18"
                height="18"
                viewBox="0 0 18 18"
                fill="none"
              >
                <path
                  d="M15.5455 3.27026C15.5455 3.07996 15.4699 2.89745 15.3353 2.76288C15.2007 2.62832 15.0182 2.55272 14.8279 2.55272H3.17211C3.04054 2.55262 2.91148 2.58869 2.79903 2.65699C2.68658 2.7253 2.59507 2.8232 2.53452 2.94001C2.47396 3.05681 2.44668 3.18802 2.45567 3.31928C2.46466 3.45054 2.50956 3.5768 2.58547 3.68426L6.81138 9.69299L6.82365 14.0645C6.825 14.3153 6.89413 14.5611 7.02372 14.7758C7.15331 14.9905 7.33854 15.1662 7.5598 15.2842C7.78107 15.4023 8.03014 15.4583 8.28065 15.4464C8.53115 15.4345 8.77378 15.3551 8.98284 15.2165L10.4924 14.2102C10.6889 14.0783 10.8497 13.8998 10.9605 13.6907C11.0713 13.4815 11.1286 13.2482 11.1273 13.0115L11.1117 9.72163L15.4129 3.68426C15.4989 3.56329 15.5452 3.41865 15.5455 3.27026ZM9.67911 9.26181L9.69629 13.0115L8.25793 13.9729L8.24484 9.23563L4.55402 3.98699H13.437L9.67911 9.26181Z"
                  fill="#485964"
                />
              </svg>
            }
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text={applyText}
            disabled={pendingAliases.length === 0}
            onClick={() => {
              if (aclAliases) {
                if (pendingSelectionCount === 0) {
                  const aliasesToApply = aclAliases
                    .filter((alias) => alias.state !== AclAliasStatus.APPLIED)
                    .map((alias) => alias.id);
                  applyPendingChangesMutation(aliasesToApply);
                } else {
                  const aliasesToApply: number[] = [];
                  for (const key in selectedPending) {
                    if (selectedPending[key]) {
                      aliasesToApply.push(Number(key));
                    }
                  }
                  applyPendingChangesMutation(aliasesToApply);
                }
              }
            }}
            loading={applyPending}
          />
          <Button
            text={localLL.listControls.addNew()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            onClick={() => {
              openCEModal();
            }}
            icon={
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="18"
                height="18"
                viewBox="0 0 18 18"
                fill="none"
              >
                <path
                  d="M4.5 9H13.5"
                  stroke="white"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
                <path
                  d="M9 4.5L9 13.5"
                  stroke="white"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </svg>
            }
          />
        </div>
      </header>
      <AliasesList
        header={{
          text: localLL.list.pendingList.title(),
        }}
        data={pendingAliases}
        noDataMessage={
          filtersPresent
            ? localLL.list.pendingList.noDataSearch()
            : localLL.list.pendingList.noData()
        }
        selected={selectedPending}
        allSelected={pendingSelectionCount === pendingAliasesCount}
        onSelect={handlePendingSelect}
        onSelectAll={handlePendingSelectAll}
      />
      <AliasesList
        isAppliedList
        header={{
          text: localLL.list.deployedList.title(),
        }}
        data={deployedAliases}
        noDataMessage={
          filtersPresent
            ? localLL.list.deployedList.noDataSearch()
            : localLL.list.deployedList.noData()
        }
      />
      <AlcAliasCEModal />
    </div>
  );
};
const prepareDisplay = (
  aclAliases: AclAliasInfo[],
  appliedFilters: AliasesFilters,
  pending: boolean,
  aclContext: Omit<AclCreateContextLoaded, 'ruleToEdit'>,
): ListData[] => {
  let aliases: AclAliasInfo[];
  let statusFilters: number[];
  let disabledStateFilter = false;
  let enabledStateFilter = false;
  if (pending) {
    aliases = aclAliases.filter((rule) => rule.state !== AclAliasStatus.APPLIED);
    statusFilters = appliedFilters.status.filter((s) => ![999, 1000].includes(s));
  } else {
    aliases = aclAliases.filter((rule) => rule.state === AclAliasStatus.APPLIED);
    statusFilters = appliedFilters.status;
    disabledStateFilter = statusFilters.includes(999);
    enabledStateFilter = statusFilters.includes(1000);
  }

  const aclStateFilter = statusFilters.map((f) => aclStatusFromInt(f));

  aliases = aliases.filter((rule) => {
    const filterChecks: boolean[] = [];
    if (statusFilters.length) {
      if (pending) {
        filterChecks.push(aclStateFilter.includes(rule.state));
      } else {
        filterChecks.push(
          (disabledStateFilter && !rule.enabled) || (enabledStateFilter && rule.enabled),
        );
      }
    }
    if (appliedFilters.networks.length && !rule.all_networks) {
      filterChecks.push(intersection(rule.networks, appliedFilters.networks).length > 0);
    }
    // if (appliedFilters.aliases.length) {
    //   filterChecks.push(intersection(rule.aliases, appliedFilters.aliases).length > 0);
    // }
    if (appliedFilters.groups.length) {
      const groups = concat(rule.denied_groups, rule.allowed_groups);
      filterChecks.push(intersection(groups, appliedFilters.groups).length > 0);
    }
    return !filterChecks.includes(false);
  });

  const listData: ListData[] = aliases.map((alias) => {

    const usedBy: ListTagDisplay[] = concat(
      alias.destination
        .split(',')
        .filter((s) => s === '')
        .map((dest, index) => ({
          key: index.toString(),
          label: dest,
          displayAsTag: false,
        })),
    );

    return {
      ...alias,
      context: {
        allowed,
        denied,
        destination: usedBy,
        networks,
      },
    };
  });

  return listData;
};
