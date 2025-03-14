import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { concat, flatten, intersection, orderBy } from 'lodash-es';
import { PropsWithChildren, ReactNode, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';

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
import { Tag } from '../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { ListSortDirection } from '../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { AclRuleInfo } from '../../../../../shared/types';
import { useAclLoadedContext } from '../../../acl-context';
import { AclStatus } from '../../../types';
import { AclIndexRulesFilterModal } from './components/AclIndexRulesFilterModal/AclIndexRulesFilterModal';
import { AclRuleStatus } from './components/AclRuleStatus/AclRuleStatus';
import { FilterDialogFilter } from './types';

type ListTagDisplay = {
  key: string | number;
  label: string;
  displayAsTag?: boolean;
};

type RulesFilters = {
  networks: number[];
  aliases: number[];
  status: number[];
};

type ListData = {
  context: {
    denied: ListTagDisplay[];
    allowed: ListTagDisplay[];
    networks: ListTagDisplay[];
    destination: ListTagDisplay[];
  };
} & Omit<AclRuleInfo, 'destination'>;

const defaultFilters: RulesFilters = {
  aliases: [],
  networks: [],
  status: [],
};

export const AclIndexRules = () => {
  const navigate = useNavigate();
  const aclContext = useAclLoadedContext();
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [appliedFilters, setAppliedFilters] = useState(defaultFilters);

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

  const pending = useMemo(() => {
    if (aclContext && aclRules) {
      let pendingRules = aclRules.filter((rule) => rule.state !== AclStatus.APPLIED);
      if (appliedFilters.networks.length > 0) {
        pendingRules = pendingRules.filter((rule) => {
          return (
            intersection(rule.networks, appliedFilters.networks).length > 0 ||
            rule.all_networks
          );
        });
      }

      const listData: ListData[] = pendingRules.map((rule) => {
        let allowed: ListTagDisplay[];
        let denied: ListTagDisplay[];
        let networks: ListTagDisplay[];

        if (rule.allow_all_users) {
          allowed = [{ key: 'all', label: 'All Allowed', displayAsTag: false }];
        } else {
          allowed = concat(
            aclContext.users
              .filter((u) => rule.allowed_users.includes(u.id))
              .map((u) => ({
                key: `user-${u.id}`,
                label: u.username,
                displayAsTag: true,
              })),
          );
        }

        if (rule.deny_all_users) {
          denied = [{ key: 'all', label: 'All Denied', displayAsTag: false }];
        } else {
          denied = concat(
            aclContext.users
              .filter((u) => rule.denied_users.includes(u.id))
              .map((user) => ({
                key: user.id,
                label: user.username,
                displayAsTag: true,
              })),
          );
        }

        if (rule.all_networks) {
          networks = [
            {
              key: 'all',
              label: 'All Included',
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
    }
    return [];
  }, [aclContext, aclRules, appliedFilters.networks]);

  const filters = useMemo(() => {
    const res: Record<string, FilterDialogFilter> = {};
    res.networks = {
      label: 'Locations',
      items: aclContext.networks.map((network) => ({
        label: network.name,
        searchValues: [network.name],
        value: network.id,
      })),
    };
    return res;
  }, [aclContext.networks]);

  const filtersCount = useMemo(() => {
    const values = flatten(Object.values(appliedFilters));
    if (values.length) {
      return ` (${values.length})`;
    }
    return '';
  }, [appliedFilters]);

  return (
    <div id="acl-rules">
      <header>
        <h2>Rules</h2>
        <div className="controls">
          <Button
            className="filter"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
            text={`Filters${filtersCount}`}
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
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Add new"
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
      {Array.isArray(pending) && (
        <RulesList
          header={{
            text: 'Pending Changes',
          }}
          data={pending}
        />
      )}
      <AclIndexRulesFilterModal
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
};

const RulesList = ({ data, header }: RulesListProps) => {
  const [sortKey, setSortKey] = useState<keyof AclRuleInfo>('name');
  const [sortDir, setSortDir] = useState<ListSortDirection>(ListSortDirection.ASC);

  const sortedRules = useMemo(
    () => orderBy(data, [sortKey], [sortDir.valueOf().toLowerCase() as 'asc' | 'desc']),
    [data, sortDir, sortKey],
  );

  const listHeaders = useMemo(
    (): ListHeaderColumnConfig<AclRuleInfo>[] => [
      {
        label: 'Rule Name',
        sortKey: 'name',
        enabled: true,
      },
      {
        label: 'Allowed',
        key: 'allowed',
        enabled: false,
      },
      {
        label: 'Denied',
        key: 'denied',
        enabled: false,
      },
      {
        label: 'Locations',
        key: 'networks',
        enabled: false,
      },
      {
        label: 'Status',
        key: 'status',
        enabled: false,
      },
      {
        label: 'Edit',
        key: 'edit',
        enabled: false,
      },
    ],
    [],
  );

  return (
    <div className="rules-list">
      <DividerHeader text={header.text}>{header.extras}</DividerHeader>
      <div className="list-container">
        <ListHeader<AclRuleInfo>
          headers={listHeaders}
          sortDirection={sortDir}
          activeKey={sortKey}
          onChange={(key, dir) => {
            setSortKey(key);
            setSortDir(dir);
          }}
        />
        <ul>
          {sortedRules.map((rule) => (
            <li key={rule.id} className="rule-row">
              <div className="cell name">{rule.name}</div>
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
          ))}
        </ul>
      </div>
    </div>
  );
};

type RenderTagsProps = {
  data: ListTagDisplay[];
};

const RenderTagDisplay = ({ data }: RenderTagsProps) => {
  return (
    <div className="tags-display">
      {data.map((d) => {
        if (d.displayAsTag) {
          return <Tag key={d.key} text={d.label} />;
        }
        return <span key={d.key}>{d.label}</span>;
      })}
    </div>
  );
};

type EditProps = {
  rule: ListData;
};

const RuleEditButton = ({ rule }: EditProps) => {
  const queryClient = useQueryClient();

  const {
    acl: {
      rules: { deleteRule },
    },
  } = useApi();

  const { mutate: deleteRuleMutation } = useMutation({
    mutationFn: deleteRule,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_ACL_RULES],
      });
    },
  });

  const navigate = useNavigate();
  return (
    <EditButton>
      <EditButtonOption
        text="Edit"
        onClick={() => {
          navigate(`/admin/acl/form?edit=1&rule=${rule.id}`);
        }}
      />
      <EditButtonOption
        text="Delete"
        styleVariant={EditButtonOptionStyleVariant.WARNING}
        onClick={() => {
          deleteRuleMutation(rule.id);
        }}
      />
    </EditButton>
  );
};
