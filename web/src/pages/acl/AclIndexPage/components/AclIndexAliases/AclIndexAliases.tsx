import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { intersection } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FilterGroupsModal } from '../../../../../shared/components/modals/FilterGroupsModal/FilterGroupsModal';
import { FilterGroupsModalFilter } from '../../../../../shared/components/modals/FilterGroupsModal/types';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ListItemCount } from '../../../../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { Search } from '../../../../../shared/defguard-ui/components/Layout/Search/Search';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useAclLoadedContext } from '../../../acl-context';
import { AclAlias, AclAliasStatus } from '../../../types';
import {
  aclAliasStatusToInt,
  aclDestinationToListTagDisplay,
  aclPortsToListTagDisplay,
  aclProtocolsToListTagDisplay,
  aclRuleToListTagDisplay,
} from '../../../utils';
import { AliasesList } from './components/AliasesList';
import { AclAliasDeleteBlockModal } from './modals/AclAliasDeleteBlockModal/AclAliasDeleteBlockModal';
import { AlcAliasCEModal } from './modals/AlcAliasCEModal/AlcAliasCEModal';
import { useAclAliasCEModal } from './modals/AlcAliasCEModal/store';
import { AclAliasListData } from './types';

type ListTagDisplay = {
  key: string | number;
  label: string;
  displayAsTag?: boolean;
};

type AliasesFilters = {
  rules: number[];
  status: number[];
};

export type ListData = {
  context: {
    usedBy: ListTagDisplay[];
  };
} & AclAlias;

const defaultFilters: AliasesFilters = {
  rules: [],
  status: [],
};

const intersects = (...args: Array<number[]>): boolean => intersection(args).length > 0;

export const AclIndexAliases = () => {
  const toaster = useToaster();
  const {
    acl: {
      rules: { getRules },
      aliases: { applyAliases },
    },
  } = useApi();

  const { data: aclRules } = useQuery({
    queryFn: getRules,
    queryKey: [QueryKeys.FETCH_ACL_RULES],
    refetchOnMount: true,
  });

  const queryClient = useQueryClient();
  const openCEModal = useAclAliasCEModal((s) => s.open, shallow);
  const aclContext = useAclLoadedContext();
  const { aliases } = aclContext;
  const [appliedFilters, setAppliedFilters] = useState<AliasesFilters>(defaultFilters);
  const filtersPresent = useMemo(
    () => Object.values(appliedFilters).flat(1).length > 0,
    [appliedFilters],
  );
  const [filtersModalOpen, setFiltersModalOpen] = useState(false);
  const [searchValue, setSearchValue] = useState('');
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases;

  const { mutate: applyMutation, isPending: applyPending } = useMutation({
    mutationFn: applyAliases,
    onSuccess: () => {
      toaster.success(LL.acl.listPage.message.applyChanges());
      void queryClient.invalidateQueries({
        predicate: (query) => query.queryKey.includes(QueryKeys.FETCH_ACL_ALIASES),
      });
    },
    onError: (error) => {
      toaster.error(LL.acl.listPage.message.applyFail());
      console.error(error);
    },
  });

  const pendingAliasesCount = useMemo(() => {
    if (aliases) {
      return aliases.filter((alias) => alias.state !== AclAliasStatus.APPLIED).length;
    }
    return 0;
  }, [aliases]);

  const applySearch = useCallback(
    (data: AclAlias[]) => {
      if (searchValue) {
        return data.filter((alias) =>
          alias.name.trim().toLowerCase().includes(searchValue.toLowerCase().trim()),
        );
      }
      return data;
    },
    [searchValue],
  );

  const [selectedPending, setSelectedPending] = useState<
    Record<number, boolean | undefined>
  >({});

  const handlePendingSelect = useCallback((key: number, value: boolean) => {
    setSelectedPending((s) => ({ ...s, [key]: value }));
  }, []);

  const handlePendingSelectAll = useCallback(
    (value: boolean, state: Record<number, boolean | undefined>) => {
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

  const prepareDisplay = useCallback(
    (data: AclAlias[], filters: AliasesFilters) => {
      if (!aclRules) return [];
      if (filters.rules.length) {
        data = data.filter((alias) => intersects(alias.rules, filters.rules));
      }
      const res: AclAliasListData[] = [];
      for (const alias of data) {
        const rules = aclRules.filter((rule) => alias.rules.includes(rule.id));
        res.push({
          ...alias,
          display: {
            destination: aclDestinationToListTagDisplay(alias.destination),
            ports: aclPortsToListTagDisplay(alias.ports),
            protocols: aclProtocolsToListTagDisplay(alias.protocols),
            rules: aclRuleToListTagDisplay(rules),
          },
        });
      }
      return res;
    },
    [aclRules],
  );

  const deployed = useMemo(() => {
    if (aliases) {
      return aliases.filter((alias) => alias.state === AclAliasStatus.APPLIED);
    }
    return [];
  }, [aliases]);

  const deployedDisplay = useMemo(() => {
    if (aliases) {
      return prepareDisplay(applySearch(deployed), appliedFilters);
    }
    return [];
  }, [aliases, prepareDisplay, applySearch, deployed, appliedFilters]);

  const pending = useMemo(() => {
    if (aliases) {
      return aliases.filter((alias) => alias.state === AclAliasStatus.MODIFIED);
    }
    return [];
  }, [aliases]);

  const pendingDisplay = useMemo(() => {
    if (aliases) {
      return prepareDisplay(applySearch(pending), appliedFilters);
    }
    return [];
  }, [aliases, appliedFilters, applySearch, pending, prepareDisplay]);

  const displayItemsCount = useMemo(
    () => deployedDisplay.length + pendingDisplay.length,
    [deployedDisplay.length, pendingDisplay.length],
  );

  const applyText = useMemo(() => {
    if (!pending.length) return localLL.listControls.apply.noChanges();
    if (pendingSelectionCount) {
      return localLL.listControls.apply.selective({
        count: pendingSelectionCount,
      });
    }
    return localLL.listControls.apply.all({
      count: pending.length,
    });
  }, [localLL.listControls.apply, pending.length, pendingSelectionCount]);

  const filters = useMemo(() => {
    const res: Record<keyof AliasesFilters, FilterGroupsModalFilter> = {
      rules: {
        label: localLL.modals.filterGroupsModal.groupLabels.rules(),
        items:
          aclRules?.map((rule) => ({
            label: rule.name,
            searchValues: [rule.name],
            value: rule.id,
          })) ?? [],
        order: 2,
      },
      status: {
        label: localLL.modals.filterGroupsModal.groupLabels.status(),
        items: [
          {
            label: localLL.list.status.changed(),
            searchValues: [LL.acl.listPage.rules.list.status.change()],
            value: aclAliasStatusToInt(AclAliasStatus.MODIFIED),
          },
          {
            label: localLL.list.status.applied(),
            searchValues: [localLL.list.status.applied()],
            value: aclAliasStatusToInt(AclAliasStatus.APPLIED),
          },
        ],
        order: 1,
      },
    };
    return res;
  }, [
    LL.acl.listPage.rules.list.status,
    aclRules,
    localLL.list.status,
    localLL.modals.filterGroupsModal.groupLabels,
  ]);

  // update or build selection state for list when rules are done loading
  useEffect(() => {
    if (aliases) {
      const pending = aliases.filter((rule) => rule.state !== AclAliasStatus.APPLIED);
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
    <>
      <AclAliasDeleteBlockModal />
      <div id="acl-aliases">
        <header>
          <h2>Aliases</h2>
          <ListItemCount count={displayItemsCount} />
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
              text={
                pendingSelectionCount > 0
                  ? localLL.listControls.filter.applied({
                      count: pendingSelectionCount,
                    })
                  : localLL.listControls.filter.nothingApplied()
              }
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.LINK}
              disabled={applyPending}
              onClick={() => {
                setFiltersModalOpen(true);
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
              disabled={pendingDisplay.length === 0}
              loading={applyPending}
              onClick={() => {
                if (aliases) {
                  if (!pendingSelectionCount) {
                    const toApply = aliases
                      .filter((alias) => alias.state === AclAliasStatus.MODIFIED)
                      .map((alias) => alias.id);
                    applyMutation(toApply);
                  } else {
                    const aliasesToApply: number[] = [];
                    for (const key in selectedPending) {
                      if (selectedPending[key]) {
                        aliasesToApply.push(Number(key));
                      }
                    }
                    applyMutation(aliasesToApply);
                  }
                }
              }}
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
          data={pendingDisplay}
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
          data={deployedDisplay}
          noDataMessage={
            filtersPresent
              ? localLL.list.deployedList.noDataSearch()
              : localLL.list.deployedList.noData()
          }
        />
        <AlcAliasCEModal />
        <FilterGroupsModal
          isOpen={filtersModalOpen}
          data={filters}
          currentState={appliedFilters}
          onSubmit={(newFilters) => {
            setAppliedFilters(newFilters as AliasesFilters);
          }}
          onCancel={() => {
            setFiltersModalOpen(false);
          }}
        />
      </div>
    </>
  );
};
