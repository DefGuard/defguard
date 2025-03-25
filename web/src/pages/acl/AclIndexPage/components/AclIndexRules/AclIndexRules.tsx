import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import clsx from 'clsx';
import { concat, intersection, orderBy } from 'lodash-es';
import {
  PropsWithChildren,
  ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { useNavigate } from 'react-router';
import { upperCaseFirst } from 'text-case';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ListHeader } from '../../../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../../../shared/components/Layout/ListHeader/types';
import { FilterGroupsModal } from '../../../../../shared/components/modals/FilterGroupsModal/FilterGroupsModal';
import { FilterGroupsModalFilter } from '../../../../../shared/components/modals/FilterGroupsModal/types';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { CheckBox } from '../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { InteractionBox } from '../../../../../shared/defguard-ui/components/Layout/InteractionBox/InteractionBox';
import { ListItemCount } from '../../../../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { NoData } from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Search } from '../../../../../shared/defguard-ui/components/Layout/Search/Search';
import { Tag } from '../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { ListSortDirection } from '../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { AclRuleInfo } from '../../../../../shared/types';
import { useAclLoadedContext } from '../../../acl-context';
import { AclCreateContextLoaded, AclStatus } from '../../../types';
import { aclStatusFromInt, aclStatusToInt } from '../../../utils';
import { AclRuleStatus } from './components/AclRuleStatus/AclRuleStatus';

type ListTagDisplay = {
  key: string | number;
  label: string;
  displayAsTag?: boolean;
};

type RulesFilters = {
  networks: number[];
  // aliases: number[];
  status: number[];
  groups: number[];
};

type ListData = {
  context: {
    denied: ListTagDisplay[];
    allowed: ListTagDisplay[];
    networks: ListTagDisplay[];
    destination: ListTagDisplay[];
  };
} & AclRuleInfo;

const defaultFilters: RulesFilters = {
  // aliases: [],
  networks: [],
  status: [],
  groups: [],
};

export const AclIndexRules = () => {
  const navigate = useNavigate();
  const {
    acl: {
      rules: { applyRules },
    },
  } = useApi();
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.rules;
  const messagesLL = LL.acl.listPage.message;
  const ruleStatusLL = localLL.list.status;
  const aclContext = useAclLoadedContext();
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [appliedFilters, setAppliedFilters] = useState(defaultFilters);
  const appliedFiltersCount = useMemo(
    () => Object.values(appliedFilters).reduce((acc, filters) => acc + filters.length, 0),
    [appliedFilters],
  );
  const [searchValue, setSearchValue] = useState('');
  const toaster = useToaster();
  const queryClient = useQueryClient();

  const { mutate: applyPendingChangesMutation, isPending: applyPending } = useMutation({
    mutationFn: applyRules,
    onSuccess: () => {
      toaster.success(messagesLL.rulesApply());
      void queryClient.invalidateQueries({
        predicate: (query) => query.queryKey.includes(QueryKeys.FETCH_ACL_RULES),
      });
    },
    onError: (e) => {
      toaster.error(messagesLL.rulesApplyFail());
      console.error(e);
    },
  });

  const {
    acl: {
      rules: { getRules },
    },
  } = useApi();

  const { data: aclRules } = useQuery({
    queryFn: getRules,
    queryKey: [QueryKeys.FETCH_ACL_RULES],
    refetchOnMount: true,
  });
  const pendingRulesCount = useMemo(() => {
    if (aclRules) {
      return aclRules.filter((rule) => rule.state !== AclStatus.APPLIED).length;
    }
    return 0;
  }, [aclRules]);

  const rulesAfterSearch = useMemo(() => {
    if (aclRules && searchValue) {
      return aclRules.filter((rule) =>
        rule.name.trim().toLowerCase().includes(searchValue.toLowerCase().trim()),
      );
    }
    return aclRules ?? [];
  }, [aclRules, searchValue]);

  const pendingRules = useMemo(
    () =>
      isPresent(aclRules)
        ? prepareDisplay(
            rulesAfterSearch,
            appliedFilters,
            localLL.list.tags.allAllowed(),
            localLL.list.tags.allDenied(),
            true,
            aclContext,
          )
        : [],
    [aclContext, aclRules, appliedFilters, localLL.list.tags, rulesAfterSearch],
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
    if (aclRules) {
      return prepareDisplay(
        rulesAfterSearch,
        appliedFilters,
        localLL.list.tags.allAllowed(),
        localLL.list.tags.allDenied(),
        false,
        aclContext,
      );
    }
    return [];
  }, [aclContext, aclRules, appliedFilters, localLL.list.tags, rulesAfterSearch]);

  const displayItemsCount = useMemo(
    () => deployedRules.length + pendingRules.length,
    [deployedRules.length, pendingRules.length],
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
    // res.aliases = {
    //   label: filterLL.alias(),
    //   order: 2,
    //   items: aclContext.aliases.map((alias) => ({
    //     label: alias.name,
    //     searchValues: [alias.name],
    //     value: alias.id,
    //   })),
    // };

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
    if (aclRules) {
      const pending = aclRules.filter((rule) => rule.state !== AclStatus.APPLIED);
      const selectionEntries = Object.keys(selectedPending).length;
      if (selectionEntries !== pending.length) {
        const newSelectionState: Record<number, boolean> = {};
        for (const rule of pending) {
          newSelectionState[rule.id] = newSelectionState[rule.id] ?? false;
        }
        setSelectedPending(newSelectionState);
      }
    }
  }, [aclRules, selectedPending]);

  return (
    <div id="acl-rules">
      <header>
        <h2>Rules</h2>
        <ListItemCount count={displayItemsCount} />
        <Search
          placeholder={localLL.listControls.searchPlaceholder()}
          initialValue={searchValue}
          onDebounce={(searchChange) => {
            setSearchValue(searchChange);
          }}
        />
        <div className="controls">
          <Button
            className="filter"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
            text={controlFilterDisplay}
            onClick={() => {
              setFiltersOpen(true);
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
            disabled={pendingRules.length === 0}
            onClick={() => {
              if (aclRules) {
                if (pendingSelectionCount === 0) {
                  const rulesToApply = aclRules
                    .filter((rule) => rule.state !== AclStatus.APPLIED)
                    .map((rule) => rule.id);
                  applyPendingChangesMutation(rulesToApply);
                } else {
                  const rulesToApply: number[] = [];
                  for (const key in selectedPending) {
                    if (selectedPending[key]) {
                      rulesToApply.push(Number(key));
                    }
                  }
                  applyPendingChangesMutation(rulesToApply);
                }
              }
            }}
            loading={applyPending}
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={localLL.listControls.addNew()}
            onClick={() => {
              navigate('/admin/acl/form');
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
      <RulesList
        header={{
          text: localLL.list.pendingList.title(),
        }}
        data={pendingRules}
        noDataMessage={
          filtersPresent
            ? localLL.list.pendingList.noDataSearch()
            : localLL.list.pendingList.noData()
        }
        selected={selectedPending}
        allSelected={pendingSelectionCount === pendingRulesCount}
        onSelect={handlePendingSelect}
        onSelectAll={handlePendingSelectAll}
      />
      <RulesList
        isAppliedList
        header={{
          text: localLL.list.deployedList.title(),
        }}
        data={deployedRules}
        noDataMessage={
          filtersPresent
            ? localLL.list.deployedList.noDataSearch()
            : localLL.list.deployedList.noData()
        }
      />
      <FilterGroupsModal
        currentState={appliedFilters}
        data={filters}
        isOpen={filtersOpen}
        onCancel={() => {
          setFiltersOpen(false);
        }}
        onSubmit={(vals) => {
          setAppliedFilters(vals as RulesFilters);
          setFiltersOpen(false);
        }}
      />
    </div>
  );
};

type DividerHeaderProps = {
  text: string;
} & PropsWithChildren;

const DividerHeader = ({ text, children }: DividerHeaderProps) => {
  return (
    <div className="divider-header spacer">
      <div className="inner">
        <p className="header">{text}</p>
        {children}
      </div>
    </div>
  );
};

type RulesListProps = {
  data: ListData[];
  header: {
    text: string;
    extras?: ReactNode;
  };
  noDataMessage: string;
  isAppliedList?: boolean;
  selected?: Record<number, boolean>;
  allSelected?: boolean;
  onSelect?: (key: number, value: boolean) => void;
  onSelectAll?: (value: boolean, state: Record<number, boolean>) => void;
};

const RulesList = ({
  data,
  header,
  noDataMessage,
  selected,
  allSelected,
  onSelect,
  onSelectAll,
}: RulesListProps) => {
  const { LL } = useI18nContext();
  const headersLL = LL.acl.listPage.rules.list.headers;
  const [sortKey, setSortKey] = useState<keyof AclRuleInfo>('name');
  const [sortDir, setSortDir] = useState<ListSortDirection>(ListSortDirection.ASC);

  const selectionEnabled = useMemo(
    () =>
      isPresent(onSelect) &&
      isPresent(onSelectAll) &&
      isPresent(selected) &&
      isPresent(allSelected),
    [onSelect, onSelectAll, selected, allSelected],
  );

  const sortedRules = useMemo(
    () => orderBy(data, [sortKey], [sortDir.valueOf().toLowerCase() as 'asc' | 'desc']),
    [data, sortDir, sortKey],
  );

  const listHeaders = useMemo(
    (): ListHeaderColumnConfig<AclRuleInfo>[] => [
      {
        label: headersLL.name(),
        sortKey: 'name',
        enabled: true,
      },
      {
        label: headersLL.allowed(),
        key: 'allowed',
        enabled: false,
      },
      {
        label: headersLL.denied(),
        key: 'denied',
        enabled: false,
      },
      {
        label: headersLL.locations(),
        key: 'networks',
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
    <div className="rules-list">
      <DividerHeader text={header.text}>{header.extras}</DividerHeader>
      {sortedRules.length === 0 && (
        <NoData customMessage={noDataMessage} messagePosition="center" />
      )}
      {sortedRules.length > 0 && (
        <div className="list-container">
          <div className={clsx('header-track')}>
            <ListHeader<AclRuleInfo>
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
            {sortedRules.map((rule) => {
              let ruleSelected = false;
              if (selected) {
                ruleSelected = selected[rule.id] ?? false;
              }
              return (
                <li
                  key={rule.id}
                  className={clsx('rule-row', {
                    selectable: selectionEnabled,
                  })}
                >
                  {selectionEnabled && (
                    <div className="cell select-cell">
                      <InteractionBox
                        onClick={() => {
                          onSelect?.(rule.id, !ruleSelected);
                        }}
                      >
                        <CheckBox value={ruleSelected} />
                      </InteractionBox>
                    </div>
                  )}
                  <div className="cell name">{upperCaseFirst(rule.name)}</div>
                  <div className="cell allowed">
                    <RenderTagDisplay data={rule.context.allowed} />
                  </div>
                  <div className="cell denied">
                    <RenderTagDisplay data={rule.context.denied} />
                  </div>
                  <div className="cell locations">
                    <RenderTagDisplay data={rule.context.networks} />
                  </div>
                  <div className="cell status">
                    <AclRuleStatus enabled={rule.enabled} status={rule.state} />
                  </div>
                  <div className="cell edit">
                    <RuleEditButton rule={rule} />
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

type RenderTagsProps = {
  data: ListTagDisplay[];
};

const RenderTagDisplay = ({ data }: RenderTagsProps) => {
  return (
    <div className="tags-display">
      <div className="track">
        {data.map((d) => {
          if (d.displayAsTag) {
            return <Tag key={d.key} text={d.label} />;
          }
          return <span key={d.key}>{d.label}</span>;
        })}
      </div>
    </div>
  );
};

type EditProps = {
  rule: ListData;
};

const RuleEditButton = ({ rule }: EditProps) => {
  const queryClient = useQueryClient();
  const isApplied = rule.state === AclStatus.APPLIED;
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.rules.list.editMenu;
  const statusLL = LL.acl.ruleStatus;
  const toaster = useToaster();

  const {
    acl: {
      rules: { deleteRule, editRule },
    },
  } = useApi();

  const invalidateQueries = useCallback(() => {
    void queryClient.invalidateQueries({
      predicate: (query) => query.queryKey.includes(QueryKeys.FETCH_ACL_RULES),
    });
  }, [queryClient]);

  const handleError = useCallback(
    (err: AxiosError) => {
      toaster.error(LL.acl.listPage.message.changeFail());
      console.error(err.message ?? err);
    },
    [LL.acl.listPage.message, toaster],
  );

  const { mutate: editRuleMutation, isPending: editPending } = useMutation({
    mutationFn: editRule,
    mutationKey: ['rule', 'delete', rule.id],
    onSuccess: () => {
      invalidateQueries();
      toaster.success(LL.acl.listPage.message.changeAdded());
    },
    onError: handleError,
  });

  const { mutate: deleteRuleMutation, isPending: deletionPending } = useMutation({
    mutationFn: deleteRule,
    onSuccess: () => {
      invalidateQueries();
      if (isApplied) {
        toaster.success(LL.acl.listPage.message.changeAdded());
      } else {
        toaster.success(LL.acl.listPage.message.changeDiscarded());
      }
    },
    onError: handleError,
  });

  const handleEnableChange = useCallback(
    (newState: boolean, rule: AclRuleInfo | ListData) => {
      editRuleMutation({ ...rule, enabled: newState, expires: rule.expires ?? null });
    },
    [editRuleMutation],
  );

  const navigate = useNavigate();

  return (
    <EditButton disabled={deletionPending || editPending}>
      <EditButtonOption
        text={LL.common.controls.edit()}
        onClick={() => {
          navigate(`/admin/acl/form?edit=1&rule=${rule.id}`);
        }}
      />
      {isApplied && (
        <>
          {!rule.enabled && (
            <EditButtonOption
              text={statusLL.enabled()}
              disabled={editPending}
              onClick={() => {
                handleEnableChange(true, rule);
              }}
            />
          )}
          {rule.enabled && (
            <EditButtonOption
              text={statusLL.disabled()}
              disabled={editPending}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => {
                handleEnableChange(false, rule);
              }}
            />
          )}
        </>
      )}
      <EditButtonOption
        disabled={deletionPending}
        text={isApplied ? localLL.delete() : localLL.discard()}
        styleVariant={EditButtonOptionStyleVariant.WARNING}
        onClick={() => {
          deleteRuleMutation(rule.id);
        }}
      />
    </EditButton>
  );
};

const prepareDisplay = (
  aclRules: AclRuleInfo[],
  appliedFilters: RulesFilters,
  allAllowedLabel: string,
  allDeniedLabel: string,
  pending: boolean,
  aclContext: Omit<AclCreateContextLoaded, 'ruleToEdit'>,
): ListData[] => {
  let rules: AclRuleInfo[];
  let statusFilters: number[];
  let disabledStateFilter = false;
  let enabledStateFilter = false;
  if (pending) {
    rules = aclRules.filter((rule) => rule.state !== AclStatus.APPLIED);
    statusFilters = appliedFilters.status.filter((s) => ![999, 1000].includes(s));
  } else {
    rules = aclRules.filter((rule) => rule.state === AclStatus.APPLIED);
    statusFilters = appliedFilters.status;
    disabledStateFilter = statusFilters.includes(999);
    enabledStateFilter = statusFilters.includes(1000);
  }

  const aclStateFilter = statusFilters.map((f) => aclStatusFromInt(f));

  rules = rules.filter((rule) => {
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

  const listData: ListData[] = rules.map((rule) => {
    let allowed: ListTagDisplay[];
    let denied: ListTagDisplay[];
    let networks: ListTagDisplay[];

    if (rule.allow_all_users) {
      allowed = [{ key: 'all', label: allAllowedLabel, displayAsTag: false }];
    } else {
      allowed = concat(
        aclContext.users
          .filter((u) => rule.allowed_users.includes(u.id))
          .map((u) => ({
            key: `user-${u.id}`,
            label: u.username,
            displayAsTag: true,
          })),
        aclContext.groups
          .filter((g) => rule.allowed_groups.includes(g.id))
          .map((group) => ({
            key: `group-${group.id}`,
            label: group.name,
            displayAsTag: true,
          })),
        aclContext.devices
          .filter((device) => rule.allowed_devices.includes(device.id))
          .map((device) => ({
            key: `device-${device.id}`,
            label: device.name,
            displayAsTag: true,
          })),
      );
    }

    if (rule.deny_all_users) {
      denied = [{ key: 'all', label: allDeniedLabel, displayAsTag: false }];
    } else {
      denied = concat(
        aclContext.users
          .filter((u) => rule.denied_users.includes(u.id))
          .map((user) => ({
            key: `user-${user.id}`,
            label: user.username,
            displayAsTag: true,
          })),
        aclContext.groups
          .filter((g) => rule.denied_groups.includes(g.id))
          .map((group) => ({
            key: `group-${group.id}`,
            label: group.name,
            displayAsTag: true,
          })),
        aclContext.devices
          .filter((device) => rule.denied_devices.includes(device.id))
          .map((device) => ({
            key: `device-${device.id}`,
            label: device.name,
            displayAsTag: true,
          })),
      );
    }

    if (rule.all_networks) {
      networks = [
        {
          key: 'all',
          label: allAllowedLabel,
        },
      ];
    } else {
      networks = aclContext.networks
        .filter((network) => rule.networks.includes(network.id))
        .map((network) => ({
          key: network.id,
          label: network.name,
          displayAsTag: true,
        }));
    }

    const destination: ListTagDisplay[] = concat(
      rule.destination
        .split(',')
        .filter((s) => s === '')
        .map((dest, index) => ({
          key: index.toString(),
          label: dest,
          displayAsTag: false,
        })),
    );

    return {
      ...rule,
      context: {
        allowed,
        denied,
        destination,
        networks,
      },
    };
  });

  return listData;
};
