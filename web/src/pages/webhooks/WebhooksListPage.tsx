import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { clone, isUndefined, orderBy } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import IconCheckmarkGreen from '../../shared/components/svg/IconCheckmarkGreen';
import IconDeactivated from '../../shared/components/svg/IconDeactivated';
import SvgIconPlusWhite from '../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../shared/constants';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { EditButton } from '../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../shared/defguard-ui/components/Layout/EditButton/types';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { ConfirmModal } from '../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { NoData } from '../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Search } from '../../shared/defguard-ui/components/Layout/Search/Search';
import { Select } from '../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../shared/defguard-ui/components/Layout/Select/types';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
} from '../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { useModalStore } from '../../shared/hooks/store/useModalStore';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { MutationKeys } from '../../shared/mutations';
import { QueryKeys } from '../../shared/queries';
import { Webhook } from '../../shared/types';
import { WebhookModal } from './modals/WebhookModal/WebhookModal';

export const WebhooksListPage = () => {
  const { LL } = useI18nContext();
  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [webhookToDelete, setWebhookToDelete] = useState<Webhook | undefined>(undefined);
  const [searchValue, setSearchValue] = useState<string>('');
  const [filteredWebhooks, setFilteredWebhooks] = useState<Webhook[]>([]);
  const setWebhookModalState = useModalStore((state) => state.setWebhookModal);

  const {
    webhook: { getWebhooks, deleteWebhook, changeWebhookState },
  } = useApi();

  const toaster = useToaster();
  const filterOptions: SelectOption<FilterOption>[] = useMemo(
    () => [
      {
        value: FilterOption.ALL,
        label: LL.webhooksOverview.filterLabels.all(),
        key: 1,
      },
      {
        value: FilterOption.ENABLED,
        label: LL.webhooksOverview.filterLabels.enabled(),
        key: 2,
      },
      {
        value: FilterOption.DISABLED,
        label: LL.webhooksOverview.filterLabels.disabled(),
        key: 3,
      },
    ],
    [LL.webhooksOverview.filterLabels],
  );
  const renderSelectedFilter = useCallback(
    (selected: FilterOption): SelectSelectedValue => {
      const option = filterOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [filterOptions],
  );
  const [selectedFilter, setSelectedFilter] = useState(FilterOption.ALL);
  const { mutate: deleteWebhookMutation, isPending: deleteWebhookIsLoading } =
    useMutation({
      mutationKey: [MutationKeys.DELETE_WEBHOOK],
      mutationFn: deleteWebhook,
      onSuccess: () => {
        toaster.success(LL.modals.deleteWebhook.messages.success());
        setDeleteModalOpen(false);
        void queryClient.invalidateQueries({
          queryKey: [QueryKeys.FETCH_WEBHOOKS],
        });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setDeleteModalOpen(false);
        console.error(err);
      },
    });

  const { mutate: changeWebhookMutation, isPending: changeWebhookIsLoading } =
    useMutation({
      mutationKey: [MutationKeys.CHANGE_WEBHOOK_STATE],
      mutationFn: changeWebhookState,
      onSuccess: () => {
        toaster.success(LL.modals.changeWebhook.messages.success());
        void queryClient.invalidateQueries({
          queryKey: [QueryKeys.FETCH_WEBHOOKS],
        });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    });

  const { data: webhooks, isLoading } = useQuery({
    queryFn: getWebhooks,
    queryKey: [QueryKeys.FETCH_WEBHOOKS],
  });

  const getHeaders = useMemo(() => {
    const res: ListHeader[] = [
      {
        key: 'url',
        text: LL.webhooksOverview.list.headers.name(),
        active: true,
        sortDirection: ListSortDirection.ASC,
      },
      {
        key: 'description',
        text: LL.webhooksOverview.list.headers.description(),
        sortable: false,
      },
      {
        key: 'status',
        text: LL.webhooksOverview.list.headers.status(),
        sortable: false,
      },
      {
        key: 'actions',
        text: LL.webhooksOverview.list.headers.actions(),
        sortable: false,
      },
    ];
    if (breakpoint !== 'desktop') {
      res.splice(1, 2);
    }
    return res;
  }, [LL.webhooksOverview.list.headers, breakpoint]);

  const getCells = useMemo(() => {
    const res: ListRowCell<Webhook>[] = [
      {
        key: 'url',
        render: (context) => <span>{context.url}</span>,
      },
      {
        key: 'description',
        render: (context) => <span>{context.description}</span>,
      },
      {
        key: 'status',
        render: (context) =>
          context.enabled ? (
            <>
              <IconCheckmarkGreen />
              <span>{LL.webhooksOverview.list.status.enabled()}</span>
            </>
          ) : (
            <>
              <IconDeactivated />
              <span>{LL.webhooksOverview.list.status.disabled()}</span>
            </>
          ),
      },
      {
        key: 'actions',
        render: (context) => (
          <EditButton>
            <EditButtonOption
              text={LL.webhooksOverview.list.editButton.edit()}
              onClick={() => setWebhookModalState({ visible: true, webhook: context })}
            />
            <EditButtonOption
              disabled={changeWebhookIsLoading}
              text={
                context.enabled
                  ? LL.webhooksOverview.list.editButton.disable()
                  : LL.webhooksOverview.list.editButton.enable()
              }
              onClick={() => {
                if (!changeWebhookIsLoading) {
                  changeWebhookMutation({
                    id: context.id,
                    enabled: !context.enabled,
                  });
                }
              }}
            />
            <EditButtonOption
              text={LL.webhooksOverview.list.editButton.delete()}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => {
                setWebhookToDelete(context);
                setDeleteModalOpen(true);
              }}
              disabled={deleteWebhookIsLoading}
            />
          </EditButton>
        ),
      },
    ];
    if (breakpoint !== 'desktop') {
      res.splice(1, 2);
    }
    return res;
  }, [
    LL.webhooksOverview.list.editButton,
    LL.webhooksOverview.list.status,
    breakpoint,
    changeWebhookIsLoading,
    changeWebhookMutation,
    deleteWebhookIsLoading,
    setWebhookModalState,
  ]);

  useEffect(() => {
    let res: Webhook[] = [];
    if (webhooks) {
      res = clone(webhooks);
      if (searchValue && searchValue.length) {
        res = res.filter((webhook) =>
          webhook.url.toLowerCase().includes(searchValue.toLowerCase()),
        );
      }
      res = orderBy(res, ['url'], ['asc']);
      switch (selectedFilter) {
        case FilterOption.ALL:
          break;
        case FilterOption.ENABLED:
          res = res.filter((r) => r.enabled);
          break;
        case FilterOption.DISABLED:
          res = res.filter((r) => !r.enabled);
          break;
        default:
          break;
      }
    }
    setFilteredWebhooks(res);
  }, [webhooks, searchValue, selectedFilter]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilter !== FilterOption.ALL) {
      setSelectedFilter(FilterOption.ALL);
    }
  }, [breakpoint, filterOptions, selectedFilter]);

  const getListPadding = useMemo(() => {
    if (breakpoint === 'desktop') {
      return {
        left: 60,
        right: 60,
      };
    }
    return {
      left: 20,
      right: 20,
    };
  }, [breakpoint]);

  return (
    <PageContainer id="webhooks-list-page">
      <header>
        <h1>{LL.webhooksOverview.pageTitle()}</h1>
        <Search
          placeholder={LL.webhooksOverview.search.placeholder()}
          initialValue={searchValue}
          debounceTiming={500}
          onDebounce={setSearchValue}
        />
      </header>
      <section className="actions">
        <div className="items-count">
          <span>{LL.webhooksOverview.webhooksCount()}</span>
          <div className="count">
            <span>{webhooks?.length ?? 0}</span>
          </div>
        </div>
        <div className="controls">
          {breakpoint === 'desktop' && (
            <Select
              options={filterOptions}
              renderSelected={renderSelectedFilter}
              selected={selectedFilter}
              onChangeSingle={(filter) => setSelectedFilter(filter)}
            />
          )}
          <Button
            className="add-item"
            onClick={() => setWebhookModalState({ visible: true, webhook: undefined })}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={
              breakpoint === 'desktop' ? LL.webhooksOverview.addNewWebhook() : undefined
            }
            icon={<SvgIconPlusWhite />}
          />
        </div>
      </section>
      {isLoading ||
        (isUndefined(webhooks) && (
          <div className="list-loader">
            <LoaderSpinner size={180} />
          </div>
        ))}
      {!isLoading && filteredWebhooks && filteredWebhooks.length === 0 && (
        <NoData customMessage={LL.webhooksOverview.noWebhooksFound()} />
      )}
      {!isLoading && filteredWebhooks && filteredWebhooks.length > 0 && (
        <VirtualizedList
          data={filteredWebhooks}
          cells={getCells}
          headers={getHeaders}
          padding={getListPadding}
          rowSize={70}
          headerPadding={{
            left: 50,
            right: 15,
          }}
        />
      )}
      <WebhookModal />
      <ConfirmModal
        title={LL.modals.deleteWebhook.title()}
        subTitle={LL.modals.deleteWebhook.message({
          name: webhookToDelete?.url || '',
        })}
        isOpen={deleteModalOpen}
        setIsOpen={setDeleteModalOpen}
        type={ConfirmModalType.WARNING}
        onSubmit={() => {
          if (!isUndefined(webhookToDelete)) {
            deleteWebhookMutation(webhookToDelete.id);
          }
        }}
        submitText={'Delete'}
        loading={deleteWebhookIsLoading}
      />
    </PageContainer>
  );
};

enum FilterOption {
  ALL = 'all',
  ENABLED = 'enabled',
  DISABLED = 'disabled',
}
